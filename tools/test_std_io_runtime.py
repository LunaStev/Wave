"""Native standard-library regressions, invoked by Cargo with its built wavec."""
import os
import errno
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest

from tools.process_tree import run_process

ROOT = Path(__file__).resolve().parent.parent
COMPILER = os.environ.get("WAVE_TEST_COMPILER")


@unittest.skipUnless(COMPILER, "Cargo supplies WAVE_TEST_COMPILER for native fixtures")
class StandardIoRuntimeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workspace = tempfile.TemporaryDirectory(prefix="wave-std-io-")
        cls.addClassCleanup(cls.workspace.cleanup)
        cls.base = Path(cls.workspace.name)
        home = cls.base / "home"
        shutil.copytree(ROOT / "std", home / ".wave/lib/wave/std")
        cls.env = dict(os.environ, HOME=str(home), USERPROFILE=str(home))
        cls.target = os.environ.get("WAVE_TEST_TARGET")
        cls.executables = {}
        names = ['copy', 'copy_self', 'read_all', 'read_zero', 'invalid_buffers']
        for name in names:
            cls.build(name)


    @classmethod
    def compile(cls, name, options):
        command = [COMPILER, "build", str(ROOT / f"tests/fixtures/io/{name}.wave")]
        if cls.target:
            command += ["--target", cls.target]
        result = run_process(command + options, env=cls.env, timeout=60,
                             capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(f"{name}: {result.stdout}\n{result.stderr}")

    @classmethod
    def build(cls, name):
        executable = cls.base / (name + (".exe" if os.name == "nt" else ""))
        cls.compile(name, ["-o", str(executable)])
        cls.executables[name] = executable

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="case-", dir=self.base)
        self.addCleanup(directory.cleanup)
        self.directory = Path(directory.name)

    def run_fixture(self, name, *args, **kwargs):
        result = run_process([str(self.executables[name]), *args], cwd=self.directory,
                             timeout=10, capture_output=True, text=True, **kwargs)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result.stdout.strip()

    def low_fd_limit(self):
        if not sys.platform.startswith("linux"):
            return {}
        def limit():
            import resource
            _, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
            resource.setrlimit(resource.RLIMIT_NOFILE, (64, hard))
        return {"preexec_fn": limit}

    def test_invalid_buffers_do_not_open_or_truncate_files(self):
        target = self.directory / "destination.bin"
        target.write_bytes(b"untouched")
        self.run_fixture("invalid_buffers")
        self.assertEqual(target.read_bytes(), b"untouched")

    def test_copy_self_preserves_bytes_and_closes_error_handles(self):
        source = self.directory / "source.bin"
        data = bytes(range(97))
        source.write_bytes(data)
        self.assertEqual(self.run_fixture("copy_self", **self.low_fd_limit()), "-4096")
        self.assertEqual(source.read_bytes(), data)

    def test_copy_hardlink_preserves_bytes(self):
        source = self.directory / "source.bin"
        data = bytes(range(97))
        source.write_bytes(data)
        os.link(source, self.directory / "destination.bin")
        self.assertEqual(self.run_fixture("copy", **self.low_fd_limit()), "-4096")
        self.assertEqual(source.read_bytes(), data)

    @unittest.skipIf(os.name == "nt", "Windows symlink creation requires a host privilege")
    def test_copy_symlink_preserves_bytes(self):
        source = self.directory / "source.bin"
        source.write_bytes(b"original")
        (self.directory / "destination.bin").symlink_to(source)
        self.assertEqual(self.run_fixture("copy"), "-4096")
        self.assertEqual(source.read_bytes(), b"original")

    def test_copy_distinct_existing_destination_is_truncated_and_handles_close(self):
        data = bytes(range(97))
        (self.directory / "source.bin").write_bytes(data)
        target = self.directory / "destination.bin"
        target.write_bytes(b"old" * 100)
        self.assertEqual(self.run_fixture("copy", **self.low_fd_limit()), "97")
        self.assertEqual(target.read_bytes(), data)

    def test_copy_creates_destination(self):
        (self.directory / "source.bin").write_bytes(b"new content")
        self.assertEqual(self.run_fixture("copy"), "11")
        self.assertEqual((self.directory / "destination.bin").read_bytes(), b"new content")

    def test_missing_copy_source_preserves_destination(self):
        target = self.directory / "destination.bin"
        target.write_bytes(b"untouched")
        self.assertLess(int(self.run_fixture("copy")), 0)
        self.assertEqual(target.read_bytes(), b"untouched")

    def test_read_empty_exact_capacity_and_oversized_files(self):
        for length in (0, 1, 127, 128, 129):
            with self.subTest(length=length):
                data = bytes(range(length))
                (self.directory / "source.bin").write_bytes(data)
                count, checksum = map(int, self.run_fixture("read_all").split())
                if length > 128:
                    self.assertEqual(count, -4098)
                else:
                    self.assertEqual(count, length)
                    self.assertEqual(checksum, sum((i+1)*b for i, b in enumerate(data)))

    def test_read_zero_capacity_probes_eof(self):
        source = self.directory / "source.bin"
        source.write_bytes(b"")
        self.assertEqual(self.run_fixture("read_zero"), "0")
        source.write_bytes(b"x")
        self.assertEqual(self.run_fixture("read_zero"), "-4098")

    @unittest.skipUnless(sys.platform.startswith("linux"), "Linux virtual file")
    def test_read_nonempty_virtual_file_with_zero_stat_size(self):
        (self.directory / "source.bin").symlink_to("/proc/self/status")
        self.assertEqual(os.stat("/proc/self/status").st_size, 0)
        self.assertEqual(self.run_fixture("read_all").split()[0], "-4098")

    @unittest.skipUnless(sys.platform.startswith("linux"), "Linux virtual file")
    def test_read_small_virtual_file_returns_actual_content(self):
        (self.directory / "source.bin").symlink_to("/proc/self/comm")
        expected = b"read_all\n"
        self.assertEqual(self.run_fixture("read_all"),
                         f"{len(expected)} {sum((i+1)*b for i,b in enumerate(expected))}")

    @unittest.skipUnless(os.name == "posix", "native FIFO")
    def test_read_stream_in_multiple_chunks_until_eof(self):
        fifo = self.directory / "source.bin"
        os.mkfifo(fifo)
        errors = []
        stopped = threading.Event()
        def produce():
            writer = None
            try:
                deadline = time.monotonic() + 5
                while not stopped.is_set():
                    try:
                        writer = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)
                        break
                    except OSError as error:
                        if error.errno != errno.ENXIO or time.monotonic() >= deadline:
                            raise
                        stopped.wait(0.005)
                if writer is not None:
                    for piece in (b"abc", b"defgh", b"ijk"):
                        os.write(writer, piece)
                        stopped.wait(0.03)
            except BaseException as error:
                errors.append(error)
            finally:
                if writer is not None:
                    os.close(writer)
        thread = threading.Thread(target=produce)
        thread.start()
        try:
            result = self.run_fixture("read_all")
        finally:
            stopped.set()
            thread.join(timeout=2)
        self.assertFalse(thread.is_alive())
        self.assertFalse(errors, errors)
        self.assertEqual(result, f"11 {sum((i+1)*b for i,b in enumerate(b'abcdefghijk'))}")












if __name__ == "__main__":
    unittest.main()

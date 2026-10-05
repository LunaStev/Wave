# SPDX-License-Identifier: MPL-2.0
"""Release notes from the exact comparison range, with commit-author credit."""
import re
from urllib.parse import quote

REPO = "wavefnd/Wave"


def pages(api, endpoint, field=None):
    page = 1
    separator = "&" if "?" in endpoint else "?"
    while True:
        result = api(f"{endpoint}{separator}per_page=100&page={page}")
        batch = result[field] if field else result
        yield from batch
        if len(batch) < 100:
            return
        page += 1


def text(value):
    return re.sub(r"[\r\n]+", " ", str(value)).replace("<", "&lt;").replace(">", "&gt;")


def contributors(pr, commits):
    people = {}
    emails = {}

    def add(user, fallback=""):
        login = user.get("login") if user else None
        if login:
            people.setdefault(login.casefold(), "@" + login)
            return "@" + login
        if fallback:
            label = text(fallback)
            people.setdefault(label.casefold(), label)
            return label

    add(pr.get("user"))
    for commit in commits:
        author = commit.get("commit", {}).get("author") or {}
        label = add(commit.get("author"), author.get("name", ""))
        if label and author.get("email"):
            emails[author["email"].casefold()] = label
    for commit in commits:
        message = commit.get("commit", {}).get("message", "")
        for name, email in re.findall(
            r"^Co-authored-by:\s*(.*?)\s*<([^<>]+)>\s*$", message, re.M | re.I
        ):
            label = emails.get(email.casefold()) or text(name)
            if label:
                people.setdefault(label.lstrip("@").casefold(), label)
    return list(people.values())


def generate(api, source, *, release_tag=None):
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValueError("release notes require an exact source SHA")
    if release_tag is not None and not re.fullmatch(
        r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", release_tag
    ):
        raise ValueError("release notes require a versioned release tag")
    # Collect changes against the verified commit; only the public link uses a tag.
    comparison_head = release_tag if release_tag is not None else source
    releases = [
        r
        for r in pages(api, "releases")
        if not r.get("draft")
        and re.fullmatch(r"v\d+\.\d+\.\d+(?:-[\w.-]+)?", r.get("tag_name", ""))
        and not r["tag_name"].endswith("-dev")
        and r["tag_name"] != release_tag
    ]
    if not releases:
        raise ValueError("no published versioned release for changelog baseline")
    # Skip a newer release outside this source's history (e.g. a delayed Nightly).
    base = None
    for release in sorted(
        releases, key=lambda r: r.get("published_at") or "", reverse=True
    ):
        candidate = release["tag_name"]
        comparison = api(f'compare/{quote(candidate, safe="")}...{source}?per_page=1')
        if comparison["status"] in ("ahead", "identical"):
            base = candidate
            break
    if base is None:
        raise ValueError("no published release is an ancestor of this source")
    commits = list(pages(api, f'compare/{quote(base, safe="")}...{source}', "commits"))
    included = {c["sha"] for c in commits}
    pulls = {}
    for commit in commits:
        for pr in pages(api, f'commits/{commit["sha"]}/pulls'):
            if (
                pr.get("merged_at")
                and pr.get("base", {}).get("ref") == "master"
                and pr.get("merge_commit_sha") in included
            ):
                pulls[pr["number"]] = pr
    lines = ["## What's Changed", ""]
    for number, pr in sorted(pulls.items()):
        authors = contributors(pr, list(pages(api, f"pulls/{number}/commits")))
        credit = " by " + ", ".join(authors) if authors else ""
        lines.append(
            f'* {text(pr["title"])}{credit} in https://github.com/{REPO}/pull/{number}'
        )
    if not pulls:
        lines.append("* No merged pull requests in this comparison range.")
    lines += [
        "",
        f'**Full Changelog**: https://github.com/{REPO}/compare/{quote(base, safe="")}...{comparison_head}',
        "",
    ]
    return "\n".join(lines)

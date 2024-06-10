package lunastev.wave.token;

public class AnalyzerException extends Exception {
    /**
     * 입력 소스 또는 토큰의 위치
     * 오류가 발생했습니다.
     */
    private int errorPosition;

    /**
     * 세부 메시지
     */
    private String message;

    /**
     * 지정된 어레 위치를 가지는 {@code AnalyzerExceptioin} 오브젝트를 작성합니다.
     *
     * @param errorPosition
     *              오류 위치
     */
    public AnalyzerException(int errorPosition) {
        this.errorPosition = errorPosition;
    }

    public AnalyzerException(String message, int errorPosition) {
        this.errorPosition = errorPosition;
        this.message = message;
    }

    public int getErrorPosition() {
        return errorPosition;
    }

    @Override
    public String getMessage() {
        return message;
    }
}

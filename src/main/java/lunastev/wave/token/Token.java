package lunastev.wave.token;

public class Token {

    private int beginIndex;

    private int endIndex;

    private TokenType tokenType;

    private String tokenString;

    public Token(int beginIndex, int endIndex, String tokenString, TokenType tokenType) {
        this.beginIndex = beginIndex;
        this.endIndex = endIndex;
        this.tokenType = tokenType;
        this.tokenString = tokenString;
    }
}

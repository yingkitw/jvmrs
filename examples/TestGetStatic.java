// Test getstatic in isolation - get System.out and discard (pop)
public class TestGetStatic {
    public static void main(String[] args) {
        Object o = System.out;  // getstatic, astore_1
        // pop would need to consume it
    }
}

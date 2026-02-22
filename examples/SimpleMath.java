public class SimpleMath {
    public static int add(int a, int b) {
        return a + b;
    }
    
    public static void main(String[] args) {
        int x = 5;
        int y = 10;
        int z = add(x, y);
        System.out.println("5 + 10 = " + z);
    }
}
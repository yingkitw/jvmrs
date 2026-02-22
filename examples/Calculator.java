public class Calculator {
    public static int add(int a, int b) {
        return a + b;
    }
    
    public static int subtract(int a, int b) {
        return a - b;
    }
    
    public static int multiply(int a, int b) {
        return a * b;
    }
    
    public static float divide(float a, float b) {
        return a / b;
    }
    
    public static void main(String[] args) {
        int x = 20;
        int y = 8;
        
        System.out.println("x = " + x);
        System.out.println("y = " + y);
        System.out.println("x + y = " + add(x, y));
        System.out.println("x - y = " + subtract(x, y));
        System.out.println("x * y = " + multiply(x, y));
        System.out.println("x / y = " + divide(x, y));
    }
}
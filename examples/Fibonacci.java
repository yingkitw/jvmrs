/**
 * Fibonacci sequence - recursive and iterative.
 * Demonstrates method calls, loops, and arithmetic.
 */
public class Fibonacci {
    public static int fibRecursive(int n) {
        if (n <= 1) return n;
        return fibRecursive(n - 1) + fibRecursive(n - 2);
    }

    public static int fibIterative(int n) {
        if (n <= 1) return n;
        int a = 0, b = 1;
        for (int i = 2; i <= n; i++) {
            int next = a + b;
            a = b;
            b = next;
        }
        return b;
    }

    public static void main(String[] args) {
        System.out.println("Fibonacci (iterative):");
        for (int i = 0; i <= 10; i++) {
            System.out.println("fib(" + i + ") = " + fibIterative(i));
        }
        System.out.println("Fibonacci (recursive, n=8): " + fibRecursive(8));
    }
}

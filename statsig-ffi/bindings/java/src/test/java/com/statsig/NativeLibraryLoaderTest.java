package com.statsig;

public class NativeLibraryLoaderTest {
    static {
        System.setProperty("java.library.path", "/Users/weihaoding/Documents/statsig-singularity/target/release");
        try {
            System.loadLibrary("statsig_ffi");
            System.out.println("Library loaded successfully.");
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Failed to load library: " + e.getMessage());
        }
    }

    public static void main(String[] args) {
        System.out.println("Library test class executed.");
    }
}

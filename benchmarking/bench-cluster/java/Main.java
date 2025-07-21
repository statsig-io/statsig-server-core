public class Main {
    public static void main(String[] args) throws Exception {
        String sdkVariant = System.getenv("SDK_VARIANT");
        
        if (sdkVariant == null) {
            throw new Exception("SDK_VARIANT is not set");
        }

        if (sdkVariant.equals("core")) {
            BenchCore.main(args);
        } else if (sdkVariant.equals("legacy")) {
            BenchLegacy.main(args);
        } else {
            throw new Exception("Invalid SDK_VARIANT: " + sdkVariant);
        }
    }
}
using System.Threading.Tasks;

namespace bench;

public class Program
{
    public static async Task Main()
    {
        var sdkVariant = Environment.GetEnvironmentVariable("SDK_VARIANT");

        if (sdkVariant == null) {
            throw new Exception("SDK_VARIANT is not set");
        }

        if (sdkVariant.Equals("core")) {
            await BenchCore.Run();
        } else if (sdkVariant.Equals("legacy")) {
            await BenchLegacy.Run();
        }
    }
}
using System.IO;

namespace Statsig.Tests;

public static class TestUtils
{
    public static string LoadJsonFile(string filename)
    {
        var currentDirectory = Directory.GetCurrentDirectory();
        var filePath = Path.Combine(currentDirectory, "Resources", filename);

        if (!File.Exists(filePath))
        {
            throw new FileNotFoundException($"Could not find file at {filePath}");
        }
        return File.ReadAllText(filePath);
    }
}
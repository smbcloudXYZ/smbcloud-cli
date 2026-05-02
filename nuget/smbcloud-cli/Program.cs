using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;

internal static class Program
{
    private const string CommandName = "smb";

    public static async Task<int> Main(string[] arguments)
    {
        try
        {
            string executablePath = ResolveExecutablePath();
            EnsureExecutablePermissions(executablePath);
            return await RunAsync(executablePath, arguments);
        }
        catch (Exception exception) when (
            exception is FileNotFoundException or
            PlatformNotSupportedException or
            Win32Exception)
        {
            Console.Error.WriteLine($"{CommandName}: {exception.Message}");
            return 1;
        }
    }

    private static string ResolveExecutablePath()
    {
        string runtimeIdentifier = GetRuntimeIdentifier();
        string executableName = OperatingSystem.IsWindows() ? $"{CommandName}.exe" : CommandName;
        string executablePath = Path.GetFullPath(
            Path.Combine(AppContext.BaseDirectory, "native", runtimeIdentifier, executableName));

        if (!File.Exists(executablePath))
        {
            throw new FileNotFoundException(
                $"The native {CommandName} executable for '{runtimeIdentifier}' is not bundled in this package.",
                executablePath);
        }

        return executablePath;
    }

    private static string GetRuntimeIdentifier()
    {
        string operatingSystem = OperatingSystem.IsWindows()
            ? "windows"
            : OperatingSystem.IsMacOS()
                ? "darwin"
                : OperatingSystem.IsLinux()
                    ? "linux"
                    : throw new PlatformNotSupportedException(
                        $"{CommandName} does not support this operating system through the .NET tool package.");

        string architecture = RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            _ => throw new PlatformNotSupportedException(
                $"{CommandName} does not support the '{RuntimeInformation.ProcessArchitecture}' architecture through the .NET tool package."),
        };

        return $"{operatingSystem}-{architecture}";
    }

    private static void EnsureExecutablePermissions(string executablePath)
    {
        if (OperatingSystem.IsWindows())
        {
            return;
        }

        UnixFileMode currentMode = File.GetUnixFileMode(executablePath);
        UnixFileMode requiredMode = UnixFileMode.UserRead |
            UnixFileMode.UserWrite |
            UnixFileMode.UserExecute |
            UnixFileMode.GroupRead |
            UnixFileMode.GroupExecute |
            UnixFileMode.OtherRead |
            UnixFileMode.OtherExecute;

        if ((currentMode & requiredMode) == requiredMode)
        {
            return;
        }

        File.SetUnixFileMode(executablePath, currentMode | requiredMode);
    }

    private static async Task<int> RunAsync(string executablePath, IReadOnlyList<string> arguments)
    {
        ProcessStartInfo startInfo = new(executablePath)
        {
            UseShellExecute = false,
            WorkingDirectory = Environment.CurrentDirectory,
        };

        foreach (string argument in arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }

        using Process process = Process.Start(startInfo)
            ?? throw new Win32Exception($"Failed to start '{CommandName}'.");

        await process.WaitForExitAsync();
        return process.ExitCode;
    }
}

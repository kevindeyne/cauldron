& "$PSScriptRoot\cauldron.exe" @args

# After any install command, refresh the current session's environment
if ($args[0] -eq "install") {
    $userPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
    $machinePath = [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
    $env:PATH = "$userPath;$machinePath"

    # Refresh known HOME vars
    foreach ($var in @("JAVA_HOME", "MAVEN_HOME", "ANT_HOME", "JMETER_HOME")) {
        $val = [System.Environment]::GetEnvironmentVariable($var, "User")
        if ($val) { Set-Item "env:$var" $val }
    }
}
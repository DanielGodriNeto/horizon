# Runs a JVM inside a Windows AppContainer (no capabilities: no network, loopback included, no access to the
# user's files) plus a Job Object (kill on close, 1 process, 1.5 GB, UI restrictions, idle priority, wall-clock
# timeout), with an empty per-run directory as cwd, temp and home. No admin needed. What it does NOT cover is in
# mods-java/POLITICA-CONFIANCA.md ("Sandbox").
#
#   sandbox.ps1 -Mode proof  -Jdk <jdk> -Shim <out-shim> -Work <dir>             (escape attempts must all FAIL)
#   sandbox.ps1 -Mode corpus -Jdk <jdk> -Shim <out-shim> -Work <dir> -Jars <dir> (LoadProbe on every jar)
param(
    [Parameter(Mandatory)][ValidateSet('proof', 'corpus')][string]$Mode,
    [Parameter(Mandatory)][string]$Jdk,
    [Parameter(Mandatory)][string]$Shim,
    [Parameter(Mandatory)][string]$Work,
    [string]$Jars,
    [string]$Root = 'C:\horizon-sbx',
    [int]$TimeoutSec = 40
)
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public static class Sbx {
    [StructLayout(LayoutKind.Sequential)] struct SECURITY_CAPABILITIES { public IntPtr Sid; public IntPtr Caps; public uint Count; public uint Reserved; }
    [StructLayout(LayoutKind.Sequential)] struct SECURITY_ATTRIBUTES { public int Length; public IntPtr Desc; public bool Inherit; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] struct STARTUPINFO {
        public int cb; public string Reserved, Desktop, Title; public int X, Y, XSize, YSize, XChars, YChars, Fill, Flags;
        public short ShowWindow, Reserved2; public IntPtr Reserved2p, StdIn, StdOut, StdErr; }
    [StructLayout(LayoutKind.Sequential)] struct STARTUPINFOEX { public STARTUPINFO Info; public IntPtr Attrs; }
    [StructLayout(LayoutKind.Sequential)] struct PROCESS_INFORMATION { public IntPtr Process, Thread; public int Pid, Tid; }
    [StructLayout(LayoutKind.Sequential)] struct JOB_EXT {
        public long PerProc, PerJob; public uint Flags; public UIntPtr MinWs, MaxWs; public uint Active; public UIntPtr Affinity;
        public uint Priority, Sched; public ulong Io1, Io2, Io3, Io4, Io5, Io6; public UIntPtr ProcMem, JobMem, PeakProc, PeakJob; }

    [DllImport("userenv.dll", CharSet = CharSet.Unicode)] static extern int CreateAppContainerProfile(string n, string d, string c, IntPtr caps, uint nc, out IntPtr sid);
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)] static extern int DeriveAppContainerSidFromAppContainerName(string n, out IntPtr sid);
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)] static extern int DeleteAppContainerProfile(string n);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool ConvertSidToStringSid(IntPtr sid, out string s);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool InitializeProcThreadAttributeList(IntPtr l, int n, int f, ref IntPtr size);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool UpdateProcThreadAttribute(IntPtr l, uint f, IntPtr a, IntPtr v, IntPtr sz, IntPtr p, IntPtr r);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool CreateProcessW(string app, StringBuilder cmd, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string cwd, ref STARTUPINFOEX si, out PROCESS_INFORMATION pi);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern IntPtr CreateFileW(string f, uint access, uint share, ref SECURITY_ATTRIBUTES sa, uint disp, uint attrs, IntPtr t);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern IntPtr CreateJobObjectW(IntPtr a, string n);
    [DllImport("kernel32.dll")] static extern bool SetInformationJobObject(IntPtr j, int cls, IntPtr info, int len);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool AssignProcessToJobObject(IntPtr j, IntPtr p);
    [DllImport("kernel32.dll")] static extern uint ResumeThread(IntPtr t);
    [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr h, uint ms);
    [DllImport("kernel32.dll")] static extern bool GetExitCodeProcess(IntPtr h, out uint code);
    [DllImport("kernel32.dll")] static extern bool TerminateJobObject(IntPtr j, uint code);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);

    static Win32Exception Fail(int step) { int e = Marshal.GetLastWin32Error(); return new Win32Exception(e, "step failed, error " + e + " at step " + step); }

    /// Creates (or finds) the AppContainer profile and returns its SID string.
    public static string Profile(string name) {
        IntPtr sid;
        int hr = CreateAppContainerProfile(name, name, "horizon mod sandbox", IntPtr.Zero, 0, out sid);
        if (hr == unchecked((int)0x800700B7)) hr = DeriveAppContainerSidFromAppContainerName(name, out sid);
        if (hr != 0) throw new Win32Exception(hr);
        string s; ConvertSidToStringSid(sid, out s); return s;
    }
    public static void DropProfile(string name) { DeleteAppContainerProfile(name); }

    /// Runs `cmd` inside the container and the job; stdout and stderr go to `log`. Returns the exit code (-1: killed on timeout).
    public static int Run(string name, string exe, string cmd, string cwd, string[] env, string log, int timeoutMs, long memBytes) {
        IntPtr sid; if (DeriveAppContainerSidFromAppContainerName(name, out sid) != 0) throw new Exception("no profile");
        IntPtr caps = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(SECURITY_CAPABILITIES)));
        Marshal.StructureToPtr(new SECURITY_CAPABILITIES { Sid = sid }, caps, false);
        IntPtr size = IntPtr.Zero; InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref size);
        IntPtr list = Marshal.AllocHGlobal(size);
        if (!InitializeProcThreadAttributeList(list, 1, 0, ref size)) throw Fail(1);
        // PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
        if (!UpdateProcThreadAttribute(list, 0, (IntPtr)0x20009, caps, (IntPtr)Marshal.SizeOf(typeof(SECURITY_CAPABILITIES)), IntPtr.Zero, IntPtr.Zero)) throw Fail(2);

        var sa = new SECURITY_ATTRIBUTES { Length = Marshal.SizeOf(typeof(SECURITY_ATTRIBUTES)), Inherit = true };
        IntPtr outH = CreateFileW(log, 0x40000000, 1, ref sa, 2, 0x80, IntPtr.Zero); // GENERIC_WRITE, share read, CREATE_ALWAYS
        if (outH == (IntPtr)(-1)) throw Fail(3);
        var si = new STARTUPINFOEX();
        si.Info.cb = Marshal.SizeOf(typeof(STARTUPINFOEX));
        si.Info.Flags = 0x100; // STARTF_USESTDHANDLES
        si.Info.StdOut = outH; si.Info.StdErr = outH; si.Info.StdIn = IntPtr.Zero;
        si.Attrs = list;

        IntPtr job = CreateJobObjectW(IntPtr.Zero, null);
        var ext = new JOB_EXT { Flags = 0x8 | 0x200 | 0x2000, Active = 1, JobMem = (UIntPtr)(ulong)memBytes };
        IntPtr ep = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(JOB_EXT)));
        Marshal.StructureToPtr(ext, ep, false);
        SetInformationJobObject(job, 9, ep, Marshal.SizeOf(typeof(JOB_EXT)));
        IntPtr ui = Marshal.AllocHGlobal(4); Marshal.WriteInt32(ui, 0xFF);
        SetInformationJobObject(job, 4, ui, 4);

        byte[] block = Encoding.Unicode.GetBytes(string.Join("\0", env) + "\0\0");
        IntPtr envp = Marshal.AllocHGlobal(block.Length);
        Marshal.Copy(block, 0, envp, block.Length);
        // EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW | IDLE_PRIORITY_CLASS
        uint flags = 0x80000 | 0x4 | 0x400 | 0x08000000 | 0x40;
        PROCESS_INFORMATION pi;
        if (!CreateProcessW(exe, new StringBuilder(cmd), IntPtr.Zero, IntPtr.Zero, true, flags, envp, cwd, ref si, out pi)) throw Fail(4);
        if (!AssignProcessToJobObject(job, pi.Process)) { TerminateJobObject(job, 1); throw Fail(5); }
        ResumeThread(pi.Thread);
        int result;
        if (WaitForSingleObject(pi.Process, (uint)timeoutMs) == 0) { uint c; GetExitCodeProcess(pi.Process, out c); result = (int)c; }
        else { TerminateJobObject(job, 99); result = -1; }
        CloseHandle(pi.Thread); CloseHandle(pi.Process); CloseHandle(job); CloseHandle(outH);
        return result;
    }
}
'@

$name = 'horizon.modsandbox'
$sid = [Sbx]::Profile($name)
$grants = @()

function Grant([string]$path, [string]$perm) {
    icacls $path /grant "*${sid}:(OI)(CI)$perm" | Out-Null
    $script:grants += $path
}

# Everything the container reads or writes lives under $Root, outside the user profile: the JVM resolves every
# parent directory of its files, and the container has no access to the profile.
function Fresh([string]$path) {
    if (Test-Path $path) { Remove-Item $path -Recurse -Force }
    New-Item -ItemType Directory $path | Out-Null
}

function Invoke-Sandboxed([string]$tag, [string]$classpath, [string[]]$javaArgs) {
    $run = Join-Path $Root "run-$tag"
    Fresh $run
    New-Item -ItemType Directory "$run\tmp", "$run\home" | Out-Null
    icacls $run /grant "*${sid}:(OI)(CI)M" | Out-Null
    $java = Join-Path $RootJdk 'bin\java.exe'
    $cmd = '"' + $java + '" -Xmx768m -Xss512k -XX:-UsePerfData -XX:TieredStopAtLevel=1 -Djava.io.tmpdir="' + "$run\tmp" +
        '" -Duser.home="' + "$run\home" + '" -cp "' + $classpath + '" ' + ($javaArgs -join ' ')
    $tmp = Join-Path $run 'tmp'
    $envBlock = @("SystemRoot=C:\Windows", "windir=C:\Windows", "SystemDrive=C:", "TEMP=$tmp", "TMP=$tmp", "USERPROFILE=$run\home", "LOCALAPPDATA=$run\home", "APPDATA=$run\home", "ProgramData=C:\ProgramData", "ALLUSERSPROFILE=C:\ProgramData", "USERNAME=sandbox", "COMPUTERNAME=sandbox", "OS=Windows_NT", "PROCESSOR_ARCHITECTURE=AMD64", "NUMBER_OF_PROCESSORS=4", "PATH=C:\Windows\System32")
    $code = [Sbx]::Run($name, $java, $cmd, $run, $envBlock, "$run\stdout.txt", $TimeoutSec * 1000, 1.5GB)
    $text = if (Test-Path "$run\stdout.txt") { Get-Content "$run\stdout.txt" -Raw } else { '' }
    [pscustomobject]@{ Code = $code; Out = $text; Run = $run }
}

New-Item -ItemType Directory $Work, $Root -Force | Out-Null
# The JDK's toRealPath lists every parent directory, and the container cannot list C:\ or the profile: a
# subst drive whose root is $Root (the only directory with an ACE for the container) makes $Root the top.
$Backing = $Root
$Drive = 'Z:'
if (Test-Path "$Drive\") { throw "$Drive is in use" }
subst $Drive $Backing
$Root = "$Drive\"
$RootJdk = Join-Path $Root 'jdk'
$RootShim = Join-Path $Root 'shim'
try {
    Fresh $RootJdk; Copy-Item (Join-Path $Jdk '*') $RootJdk -Recurse -Force
    Fresh $RootShim; Copy-Item (Join-Path $Shim '*') $RootShim -Recurse -Force
    Grant $RootJdk 'RX'
    Grant $RootShim 'RX'
    Grant $Root 'RX'
    if ($Mode -eq 'proof') {
        $proof = Join-Path $Root 'proof-classes'
        Fresh $proof
        & (Join-Path $RootJdk 'bin\javac.exe') --release 25 -d $proof (Join-Path $PSScriptRoot 'ProofProbe.java')
        Grant $proof 'RX'
        $outside = Join-Path $Work 'outside'
        New-Item -ItemType Directory $outside -Force | Out-Null
        Set-Content "$outside\secret.txt" 'secret'
        $r = Invoke-Sandboxed 'proof' $proof @('ProofProbe', "`"$outside`"")
        $r.Out
        "exit=$($r.Code) pwned.txt exists outside: $(Test-Path "$outside\pwned.txt"); proof-write in run dir: $(Test-Path "$($r.Run)\proof-write.txt")"
    } else {
        $jarDir = Join-Path $Root 'jars'
        Fresh $jarDir
        Copy-Item (Join-Path $Jars '*.jar') $jarDir -Force
        Grant $jarDir 'RX'
        $i = 0
        foreach ($jar in Get-ChildItem $jarDir -Filter *.jar | Sort-Object Name) {
            $i++
            $r = Invoke-Sandboxed "j$i" $RootShim @('horizon.LoadProbe', "`"$($jar.FullName)`"")
            foreach ($line in ($r.Out -split "`r?`n")) { if ($line) { "$($jar.Name)`t$($r.Code)`t$line" } }
        }
    }
} finally {
    foreach ($g in $grants) { icacls $g /remove "*$sid" | Out-Null }
    [Sbx]::DropProfile($name)
    subst $Drive /d
    Remove-Item $Backing -Recurse -Force -ErrorAction SilentlyContinue
}

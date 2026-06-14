Option Explicit

Dim fso, shell, baseDir, appPath, logPath
Set fso = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")

baseDir = fso.GetParentFolderName(WScript.ScriptFullName)
appPath = fso.BuildPath(baseDir, "app.py")
logPath = shell.ExpandEnvironmentStrings("%TEMP%") & "\ysn_deploy_helper_launch.log"
shell.CurrentDirectory = baseDir

Sub WriteLog(message)
  On Error Resume Next
  Dim file
  Set file = fso.OpenTextFile(logPath, 8, True)
  file.WriteLine Now & " " & message
  file.Close
  On Error GoTo 0
End Sub

Function Q(value)
  Q = Chr(34) & value & Chr(34)
End Function

Function CleanPathPart(value)
  value = Trim(value)
  If Len(value) >= 2 Then
    If Left(value, 1) = Chr(34) And Right(value, 1) = Chr(34) Then
      value = Mid(value, 2, Len(value) - 2)
    End If
  End If
  CleanPathPart = value
End Function

Function FindOnPath(fileName)
  Dim pathValue, parts, folder, candidate
  pathValue = shell.ExpandEnvironmentStrings("%PATH%")
  parts = Split(pathValue, ";")
  For Each folder In parts
    folder = CleanPathPart(folder)
    If Len(folder) > 0 Then
      candidate = fso.BuildPath(folder, fileName)
      If fso.FileExists(candidate) Then
        FindOnPath = candidate
        Exit Function
      End If
    End If
  Next
  FindOnPath = ""
End Function

Dim pythonExe, pythonArgs, launchExe, launchArgs, pythonwExe, pywExe, rc, launchCmd
pythonExe = FindOnPath("python.exe")
pythonArgs = ""
If pythonExe = "" Then
  pythonExe = FindOnPath("py.exe")
  pythonArgs = " -3"
End If

If pythonExe = "" Then
  MsgBox "Python was not found. Please install Python 3 and add it to PATH.", 16, "YSN Deploy Helper"
  WScript.Quit 1
End If

If Not fso.FileExists(appPath) Then
  MsgBox "app.py was not found next to this launcher.", 16, "YSN Deploy Helper"
  WScript.Quit 1
End If

WriteLog "Using Python: " & pythonExe

rc = shell.Run(Q(pythonExe) & pythonArgs & " -c " & Q("import webview"), 0, True)
If rc <> 0 Then
  WriteLog "pywebview is missing; trying pip install pywebview"
  rc = shell.Run(Q(pythonExe) & pythonArgs & " -m pip install pywebview", 0, True)
  If rc <> 0 Then
    WriteLog "pywebview install failed with code " & CStr(rc)
    MsgBox "pywebview install failed. See log: " & logPath, 16, "YSN Deploy Helper"
    WScript.Quit rc
  End If
End If

launchExe = pythonExe
launchArgs = pythonArgs
If LCase(fso.GetFileName(pythonExe)) = "python.exe" Then
  pythonwExe = fso.BuildPath(fso.GetParentFolderName(pythonExe), "pythonw.exe")
  If fso.FileExists(pythonwExe) Then
    launchExe = pythonwExe
    launchArgs = ""
  End If
Else
  pywExe = FindOnPath("pyw.exe")
  If pywExe <> "" Then
    launchExe = pywExe
    launchArgs = " -3"
  End If
End If

launchCmd = Q(launchExe) & launchArgs & " " & Q(appPath)
WriteLog "Launch command: " & launchCmd

If shell.ExpandEnvironmentStrings("%YSN_DEPLOY_LAUNCHER_DRY_RUN%") = "1" Then
  WriteLog "Dry run ok"
  WScript.Quit 0
End If

rc = shell.Run(launchCmd, 0, False)
WScript.Quit rc

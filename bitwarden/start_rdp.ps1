$Username = "kramer9"
$Password = ConvertTo-SecureString "Adminpassw0rd" -AsPlainText -Force
$Credential = New-Object System.Management.Automation.PSCredential($Username, $Password)
$RemoteComputer = "20.121.136.210"
cmdkey /generic:TERMSRV/$RemoteComputer /user:$Username /pass:$Password
Start-Process "mstsc.exe" -ArgumentList "/v:$RemoteComputer"
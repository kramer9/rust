use clap::Parser;
use std::process::{Command, Stdio};
use std::io::{self, Write};
use std::fs::File;
use std::thread;
use std::time::Duration;
use rpassword::read_password;
use serde_json::Value;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// Target connection type: "rdp" or "mputty"
    #[clap(short, long)]
    target: String,

    /// Bitwarden item name or id to retrieve the secret
    #[clap(short = 'e', long)]
    secret: String,
}

/// Syncs the Bitwarden vault.
fn sync_bitwarden() {
    let output = Command::new("bw")
        .arg("sync")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute Bitwarden CLI");

    if output.status.success() {
        println!("Bitwarden vault synced successfully.");
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error syncing Bitwarden vault: {}", err);
    }
}

/// Retrieves the specified field using the Bitwarden CLI ("bw").
fn retrieve_secret(secret_id: &str, field: &str, session: &str) -> Option<String> {
    let output = Command::new("bw")
        .args(&["get", "item", secret_id])
        .env("BW_SESSION", session)
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute Bitwarden CLI");

    if output.status.success() {
        let item = String::from_utf8_lossy(&output.stdout);
        let item_json: Value = serde_json::from_str(&item).expect("Failed to parse JSON");

        if field == "username" || field == "password" {
            item_json["login"][field].as_str().map(|s| s.to_string())
        } else if field == "uri" {
            item_json["login"]["uris"].as_array()
                .and_then(|uris| uris.get(0))
                .and_then(|uri| uri["uri"].as_str())
                .map(|s| s.to_string())
        } else {
            item_json["fields"].as_array()
                .and_then(|fields| fields.iter()
                    .find(|f| f["name"] == field)
                    .and_then(|f| f["value"].as_str())
                    .map(|s| s.to_string()))
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error retrieving field '{}' from secret '{}': {}", field, secret_id, err);
        None
    }
}

/// Unlocks the Bitwarden CLI and returns the session key.
fn unlock_bitwarden() -> Option<String> {
    print!("Master password: ");
    io::stdout().flush().unwrap();
    let master_password = read_password().expect("Failed to read password");

    let mut child = Command::new("bw")
        .args(&["unlock", "--raw"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute Bitwarden CLI");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(master_password.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");

    if output.status.success() {
        let session_key = String::from_utf8_lossy(&output.stdout);
        Some(session_key.trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error unlocking Bitwarden: {}", err);
        None
    }
}

fn main() {
    let args = Args::parse();

    // Sync Bitwarden vault.
    sync_bitwarden();

    // Unlock Bitwarden and get the session key.
    let session_key = unlock_bitwarden().expect("Failed to unlock Bitwarden");

    // Retrieve credentials from Bitwarden.
    let username = retrieve_secret(&args.secret, "username", &session_key)
        .expect("Unable to retrieve username from Bitwarden");
    let password = retrieve_secret(&args.secret, "password", &session_key)
        .expect("Unable to retrieve password from Bitwarden");
    let uri = retrieve_secret(&args.secret, "uri", &session_key)
        .unwrap_or_else(|| "localhost".to_string());

    // Display the retrieved fields.
    println!("Username: {}", username);
    println!("Password: {}", password);
    println!("URI: {}", uri);

    // Escape special characters in the password for PowerShell.
    let escaped_password = password.replace("`", "``").replace("\"", "`\"").replace("$", "`$").replace("!", "`!");

    // Create a PowerShell script to store credentials and start RDP session.
    let powershell_script_content = format!(
        "$Username = \"{}\"\n\
        $Password = ConvertTo-SecureString \"{}\" -AsPlainText -Force\n\
        $Credential = New-Object System.Management.Automation.PSCredential($Username, $Password)\n\
        $RemoteComputer = \"{}\"\n\
        cmdkey /generic:TERMSRV/$RemoteComputer /user:$Username /pass:$Password\n\
        Start-Process \"mstsc.exe\" -ArgumentList \"/v:$RemoteComputer\"",
        username, escaped_password, uri
    );

    let powershell_script_path = "start_rdp.ps1";
    {
        let mut file = File::create(powershell_script_path).expect("Failed to create PowerShell script file");
        file.write_all(powershell_script_content.as_bytes()).expect("Failed to write PowerShell script file");
    }

    // Ensure PowerShell execution policy allows script execution.
    let execution_policy_output = Command::new("powershell")
        .arg("-Command")
        .arg("Set-ExecutionPolicy RemoteSigned -Scope Process -Force")
        .output()
        .expect("Failed to set PowerShell execution policy");

    if !execution_policy_output.status.success() {
        let err = String::from_utf8_lossy(&execution_policy_output.stderr);
        eprintln!("Failed to set PowerShell execution policy: {}", err);
        return;
    }

    // Add a small delay to ensure the file system releases the file handle.
    thread::sleep(Duration::from_millis(100));

    // Execute the PowerShell script.
    println!("Executing PowerShell script: {}", powershell_script_path);
    let powershell_output = Command::new("powershell")
        .arg("-File")
        .arg(powershell_script_path)
        .output()
        .expect("Failed to execute PowerShell script");

    if !powershell_output.status.success() {
        let err = String::from_utf8_lossy(&powershell_output.stderr);
        eprintln!("Failed to execute PowerShell script: {}", err);
    } else {
        let powershell_stdout = String::from_utf8_lossy(&powershell_output.stdout);
        println!("PowerShell script executed successfully. Output:\n{}", powershell_stdout);
    }
}
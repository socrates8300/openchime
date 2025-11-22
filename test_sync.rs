// Test script to debug sync issues
use std::process::Command;

fn main() {
    println!("Testing OpenChime sync functionality...");
    
    // Test 1: Check if we can fetch ICS data
    println!("\n1. Testing ICS fetch...");
    let output = Command::new("curl")
        .arg("-s")
        .arg("https://www.calendarlabs.com/ical-calendar/ics/76/US_Holidays.ics")
        .arg("|")
        .arg("head")
        .arg("-n")
        .arg("5")
        .output();
    
    match output {
        Ok(result) => {
            println!("Status: {}", result.status);
            println!("Output: {}", String::from_utf8_lossy(&result.stdout));
            println!("Error: {}", String::from_utf8_lossy(&result.stderr));
        }
        Err(e) => {
            println!("Failed to run curl: {}", e);
        }
    }
    
    // Test 2: Check if we can run OpenChime and see logs
    println!("\n2. Testing OpenChime startup...");
    let output = Command::new("timeout")
        .arg("15s")
        .arg("cargo")
        .arg("run")
        .output();
    
    match output {
        Ok(result) => {
            println!("Status: {}", result.status);
            println!("Stdout: {}", String::from_utf8_lossy(&result.stdout));
            println!("Stderr: {}", String::from_utf8_lossy(&result.stderr));
        }
        Err(e) => {
            println!("Failed to run OpenChime: {}", e);
        }
    }
}
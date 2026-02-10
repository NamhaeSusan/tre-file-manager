use std::io::{self, Write};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

fn main() {
    eprint!("Enter password: ");
    io::stderr().flush().unwrap();

    let mut password = String::new();
    io::stdin().read_line(&mut password).unwrap();
    let password = password.trim();

    if password.is_empty() {
        eprintln!("Password cannot be empty");
        std::process::exit(1);
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password");

    println!("{hash}");
}

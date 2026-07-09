use std::io::{self, Write};

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, EntityTrait, QueryFilter};

use my_retreat_nest::entities_helper::{
    AdminUserActiveModel, UserActiveModel, UserColumn, UserEntity,
};
use my_retreat_nest::env::ENV;
use my_retreat_nest::utils::password::create_password;

fn prompt(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

#[tokio::main]
async fn main() {
    let _ = &*ENV;

    println!("=== Create Super Admin ===");

    let name = prompt("Name: ");
    let email = prompt("Email: ");
    let password = prompt("Password: ");
    let password_confirm = prompt("Confirm password: ");
    let phone = prompt("Phone (optional): ");

    if name.is_empty() {
        eprintln!("Error: Name is required");
        std::process::exit(1);
    }
    if email.is_empty() || !email.contains('@') {
        eprintln!("Error: Valid email is required");
        std::process::exit(1);
    }
    if password.len() < 6 {
        eprintln!("Error: Password must be at least 6 characters");
        std::process::exit(1);
    }
    if password != password_confirm {
        eprintln!("Error: Passwords do not match");
        std::process::exit(1);
    }

    let db = Database::connect(&ENV.database_url)
        .await
        .expect("Failed to connect to database");

    let existing = UserEntity::find()
        .filter(UserColumn::Email.eq(email.clone()))
        .one(&db)
        .await
        .expect("Failed to query user");

    if let Some(user) = existing {
        eprintln!(
            "Error: User with email '{}' already exists (user_id={})",
            email, user.user_id
        );
        std::process::exit(1);
    }

    let hashed_password = create_password(&password)
        .await
        .expect("Failed to hash password");

    let user = UserActiveModel {
        name: Set(name.clone()),
        email: Set(email.clone()),
        password: Set(hashed_password),
        phone: Set(if phone.is_empty() {
            None
        } else {
            Some(phone)
        }),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Failed to create user");

    println!("✅ Created user: {} (user_id={})", user.name, user.user_id);

    let admin = AdminUserActiveModel {
        user_id: Set(user.user_id),
        role: Set("superadmin".to_string()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Failed to create admin user");

    println!("✅ Created superadmin (admin_user_id={})", admin.admin_user_id);
    println!("\nSuper admin '{}' created successfully!", user.name);
}

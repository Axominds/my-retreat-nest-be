use lettre::{
    AsyncTransport, Message,
    message::header::ContentType,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParametersBuilder},
    },
    AsyncSmtpTransport, Tokio1Executor,
};

use crate::env;

pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl std::fmt::Debug for Mailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mailer").field("from", &self.from).finish()
    }
}

impl Mailer {
    pub fn new() -> Self {
        let creds = Credentials::new(env::ENV.smtp_username.clone(), env::ENV.smtp_password.clone());

        let tls_params = TlsParametersBuilder::new(env::ENV.smtp_host.clone())
            .build()
            .expect("Failed to build TLS parameters");

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&env::ENV.smtp_host)
            .expect("Failed to create SMTP transport")
            .port(env::ENV.smtp_port)
            .tls(Tls::Required(tls_params))
            .credentials(creds)
            .build();

        Self {
            transport,
            from: env::ENV.smtp_username.clone(),
        }
    }

    pub async fn send_listing_approved_email(
        &self,
        to_email: &str,
        owner_name: &str,
        retreat_name: &str,
        temp_password: Option<&str>,
    ) {
        let from_addr = match self.from.parse() {
            Ok(addr) => addr,
            Err(e) => {
                tracing::error!("Invalid from address '{}': {}", self.from, e);
                return;
            }
        };
        let to_addr = match to_email.parse() {
            Ok(addr) => addr,
            Err(e) => {
                tracing::error!("Invalid to address '{}': {}", to_email, e);
                return;
            }
        };

        let (subject, html_body) = if let Some(password) = temp_password {
            (
                format!("Your Retreat '{}' Has Been Approved! – My Retreat Nest", retreat_name),
                format!(
                    r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: Arial, sans-serif; padding: 20px;">
    <h2>Your Retreat Has Been Listed!</h2>
    <p>Dear {},</p>
    <p>Congratulations! Your retreat <strong>{}</strong> has been reviewed and approved. It is now live on My Retreat Nest.</p>
    <p>An owner account has been created for you:</p>
    <table style="background: #f9f9f9; padding: 16px; border-radius: 8px; margin: 16px 0;">
        <tr><td><strong>Email:</strong></td><td>{}</td></tr>
        <tr><td><strong>Temp Password:</strong></td><td><code>{}</code></td></tr>
    </table>
    <p>Please log in and change your password as soon as possible.</p>
    <a href="{}" style="display: inline-block; padding: 12px 24px; background-color: #16a34a; color: white; text-decoration: none; border-radius: 6px; font-size: 16px; margin-top: 8px;">Log In to Your Account</a>
    <p style="margin-top: 24px;">You can manage your retreat from your owner dashboard.</p>
</body>
</html>"#,
                    owner_name, retreat_name, to_email, password, env::ENV.app_url
                ),
            )
        } else {
            (
                format!("Your Retreat '{}' Has Been Approved! – My Retreat Nest", retreat_name),
                format!(
                    r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: Arial, sans-serif; padding: 20px;">
    <h2>Your Retreat Has Been Listed!</h2>
    <p>Dear {},</p>
    <p>Congratulations! Your retreat <strong>{}</strong> has been reviewed and approved. It is now live on My Retreat Nest.</p>
    <p>You can view and manage your retreat from your account dashboard.</p>
    <a href="{}/login" style="display: inline-block; padding: 12px 24px; background-color: #16a34a; color: white; text-decoration: none; border-radius: 6px; font-size: 16px; margin-top: 8px;">Log In to Your Account</a>
    <p style="margin-top: 24px;">You have been added as an owner of this retreat.</p>
</body>
</html>"#,
                    owner_name, retreat_name, env::ENV.app_url
                ),
            )
        };

        let email = match Message::builder()
            .from(from_addr)
            .to(to_addr)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body)
        {
            Ok(email) => email,
            Err(e) => {
                tracing::error!("Failed to build listing approved email for {}: {}", to_email, e);
                return;
            }
        };

        if let Err(e) = self.transport.send(email).await {
            tracing::error!("Failed to send listing approved email to {}: {}", to_email, e);
        }
    }

    pub async fn send_reset_email(&self, to_email: &str, token: &str) {
        let reset_url = format!("{}/reset-password?token={}", env::ENV.app_url, token);

        let from_addr = match self.from.parse() {
            Ok(addr) => addr,
            Err(e) => {
                tracing::error!("Invalid from address '{}': {}", self.from, e);
                return;
            }
        };
        let to_addr = match to_email.parse() {
            Ok(addr) => addr,
            Err(e) => {
                tracing::error!("Invalid to address '{}': {}", to_email, e);
                return;
            }
        };

        let html_body = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: Arial, sans-serif; padding: 20px;">
    <h2>Password Reset</h2>
    <p>You requested a password reset. Click the button below to reset your password:</p>
    <a href="{}" style="display: inline-block; padding: 12px 24px; background-color: #4F46E5; color: white; text-decoration: none; border-radius: 6px; font-size: 16px;">Reset Password</a>
    <p style="margin-top: 20px;">If you did not request this, please ignore this email.</p>
    <p>This link expires in 1 hour.</p>
</body>
</html>"#,
            reset_url
        );

        let email = match Message::builder()
            .from(from_addr)
            .to(to_addr)
            .subject("Password Reset - My Retreat Nest")
            .header(ContentType::TEXT_HTML)
            .body(html_body)
        {
            Ok(email) => email,
            Err(e) => {
                tracing::error!("Failed to build reset email for {}: {}", to_email, e);
                return;
            }
            };

        if let Err(e) = self.transport.send(email).await {
            tracing::error!("Failed to send reset email to {}: {}", to_email, e);
        }
    }
}

impl Clone for Mailer {
    fn clone(&self) -> Self {
        Self::new()
    }
}

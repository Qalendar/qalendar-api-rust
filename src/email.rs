use lettre::{
    message::{header::ContentType, Mailbox, Message},
    transport::smtp::{authentication::Credentials, client::{Tls, TlsParameters}},
    SmtpTransport, Transport, Address,
};
use crate::config::Config;
use crate::errors::AppError; // Use AppError for email sending errors
use std::sync::Arc; // For Arc<Config>
use rustls::ClientConfig as RustlsClientConfig;
use urlencoding; // For URL encoding

#[derive(Clone)] // EmailService needs to be cloneable to be in AppState
pub struct EmailService {
    sender: Mailbox,
    mailer: SmtpTransport,
    config: Arc<Config>, 
}

impl EmailService {
    pub fn new(config: Arc<Config>) -> Result<Self, AppError> {
        let sender_email: Address = config.sender_email.parse()
            .map_err(|e| AppError::ConfigurationError(format!("Invalid sender email address: {}", e)))?;
        let sender_name = config.sender_name.clone();
        let sender = Mailbox::new(Some(sender_name), sender_email);

        let creds = Credentials::new(config.smtp_user.clone(), config.smtp_password.clone());

        // Configure TLS parameters
        // let root_store = rustls::RootCertStore::from_iter(
        //     webpki_roots::TLS_SERVER_ROOTS
        //         .iter()
        //         .cloned(),
        // );
        // let rustls_config = RustlsClientConfig::builder()
        //     .with_root_certificates(root_store)
        //     .with_no_client_auth();
        let tls_parameters = TlsParameters::new(config.smtp_server.clone()) // Pass the SMTP server domain
            .map_err(|e| AppError::ConfigurationError(format!("Failed to create TLS parameters: {}", e)))?;

        // Use relay for SMTP server
        let mailer = SmtpTransport::relay(&config.smtp_server)
            .map_err(|e| AppError::ConfigurationError(format!("Failed to create SMTP transport: {}", e)))?
            .port(config.smtp_port)
            .credentials(creds)
            .tls(Tls::Required(tls_parameters))
            .build();

        Ok(Self { sender, mailer, config })
    }

    // Send a verification email (Modified)
    pub async fn send_verification_email(&self, recipient_email: &str, verification_code: &str) -> Result<(), AppError> {
        let recipient_address: Address = recipient_email.parse()
             .map_err(|e| AppError::EmailSendingError(format!("Invalid recipient email address: {}", e)))?;

        // --- Use frontend_url from config ---
        let verification_link = format!(
            "{}/verify-email?email={}&code={}", // Use the configurable URL
            self.config.frontend_url,
            urlencoding::encode(recipient_email), // URL-encode email
            urlencoding::encode(verification_code) // URL-encode code
        );

        let email_body = format!(
            "Hi,\n\nPlease click the following link to verify your email address for Qalendar: \n{}\n\nThis link expires in {} minutes.\n\nIf you did not sign up for Qalendar, please ignore this email.",
            verification_link,
            self.config.verification_code_expires_minutes // Use expiry from config
        );

        let email = Message::builder()
            .from(self.sender.clone())
            .to(Mailbox::new(None, recipient_address))
            .subject("Verify Your Qalendar Email Address")
            .header(ContentType::TEXT_PLAIN)
            .body(email_body)
            .map_err(|e| AppError::EmailSendingError(format!("Failed to build email message: {}", e)))?;


        // Sending happens in a blocking context because SmtpTransport::send is not async
        let mailer = self.mailer.clone(); // Clone mailer for the blocking task
        tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| AppError::InternalServerError(format!("Email sending task failed: {}", e)))? // Handle join error
            .map_err(|e| AppError::EmailSendingError(format!("Failed to send email: {:?}", e)))?; // Handle send error

        tracing::info!("Verification email sent to {}", recipient_email);
        Ok(())
    }

    // Send a password reset email
    pub async fn send_password_reset_email(&self, recipient_email: &str, reset_code: &str) -> Result<(), AppError> {
        let recipient_address: Address = recipient_email.parse()
            .map_err(|e| AppError::EmailSendingError(format!("Invalid recipient email address: {}", e)))?;

        // --- Use frontend_url from config ---
        let reset_link = format!(
            "{}/reset-password?email={}&code={}", // Use the configurable URL
            self.config.frontend_url,
            urlencoding::encode(recipient_email), // URL-encode email
            urlencoding::encode(reset_code) // URL-encode code
        );

        let email_body = format!(
            "Hi,\n\nYou requested a password reset for your Qalendar account. Click the link below to reset your password:\n{}\n\nThis link expires in {} minutes.\n\nIf you did not request a password reset, please ignore this email.",
            reset_link,
            self.config.reset_code_expires_minutes // Use expiry from config
        );

        let email = Message::builder()
            .from(self.sender.clone())
            .to(Mailbox::new(None, recipient_address))
            .subject("Reset Your Qalendar Password")
            .header(ContentType::TEXT_PLAIN)
            .body(email_body)
            .map_err(|e| AppError::EmailSendingError(format!("Failed to build email message: {}", e)))?;


        let mailer = self.mailer.clone(); // Clone mailer for the blocking task
        tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| AppError::InternalServerError(format!("Email sending task failed: {}", e)))?
            .map_err(|e| AppError::EmailSendingError(format!("Failed to send email: {:?}", e)))?;

        tracing::info!("Password reset email sent to {}", recipient_email);
        Ok(())
    }

    // Add other email sending methods here (e.g., for event invitations) LATER
    // pub async fn send_invitation_email(...) -> Result<(), AppError> { ... }
}
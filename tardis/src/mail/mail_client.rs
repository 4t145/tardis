use std::collections::HashMap;

use lettre::message::{header, MultiPart, SinglePart};
use lettre::transport::smtp::client::{Tls, TlsParametersBuilder, TlsVersion};
use lettre::{address, error, transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{error, info, trace, warn};

use crate::basic::error::TardisError;
use crate::{FrameworkConfig, TardisFuns, TardisResult};

pub struct TardisMailClient {
    client: AsyncSmtpTransport<Tokio1Executor>,
    default_from: String,
}

impl TardisMailClient {
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<HashMap<String, TardisMailClient>> {
        let mut clients = HashMap::new();
        clients.insert(
            "".to_string(),
            TardisMailClient::init(
                &conf.mail.smtp_host,
                conf.mail.smtp_port,
                &conf.mail.smtp_username,
                &conf.mail.smtp_password,
                &conf.mail.default_from,
                conf.mail.starttls,
            )?,
        );
        for (k, v) in &conf.mail.modules {
            clients.insert(
                k.to_string(),
                TardisMailClient::init(&v.smtp_host, v.smtp_port, &v.smtp_username, &v.smtp_password, &v.default_from, v.starttls)?,
            );
        }
        Ok(clients)
    }

    pub fn init(smtp_host: &str, smtp_port: u16, smtp_username: &str, smtp_password: &str, default_from: &str, starttls: bool) -> TardisResult<TardisMailClient> {
        info!("[Tardis.MailClient] Initializing");
        let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());
        let tls = TlsParametersBuilder::new(smtp_host.to_string())
            .dangerous_accept_invalid_certs(true)
            .dangerous_accept_invalid_hostnames(true)
            .set_min_tls_version(TlsVersion::Tlsv10)
            .build()
            .map_err(|error| TardisError::internal_error(&format!("[Tardis.MailClient] Tls build error: {error}"), "500-tardis-mail-init-error"))?;
        let (client, tls) = if starttls {
            info!("[Tardis.MailClient] Using STARTTLS");
            (AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host), Tls::Opportunistic(tls))
        } else {
            (AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host), Tls::Wrapper(tls))
        };
        let client = client
            .map_err(|_| TardisError::internal_error(&format!("[Tardis.MailClient] Failed to create SMTP client: {smtp_host}"), "500-tardis-mail-init-error"))?
            .credentials(creds)
            .tls(tls)
            .port(smtp_port)
            .build();
        info!("[Tardis.MailClient] Initialized");
        TardisResult::Ok(TardisMailClient {
            client,
            default_from: default_from.to_string(),
        })
    }

    pub async fn send(&self, req: &TardisMailSendReq) -> TardisResult<()> {
        let mut email = Message::builder();
        email = if let Some(from) = &req.from {
            email.from(from.parse()?)
        } else {
            email.from(self.default_from.as_str().parse()?)
        };
        for to in &req.to {
            email = email.to(to.parse()?)
        }
        if let Some(reply_to) = &req.reply_to {
            for t in reply_to {
                email = email.reply_to(t.parse()?)
            }
        };
        if let Some(cc) = &req.cc {
            for t in cc {
                email = email.cc(t.parse()?)
            }
        };
        if let Some(bcc) = &req.bcc {
            for t in bcc {
                email = email.bcc(t.parse()?)
            }
        };
        email = email.subject(&req.subject);
        let email = if let Some(html_body) = &req.html_body {
            email.multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::builder().header(header::ContentType::TEXT_PLAIN).body(req.txt_body.clone()))
                    .singlepart(SinglePart::builder().header(header::ContentType::TEXT_HTML).body(html_body.to_string())),
            )?
        } else {
            email.header(header::ContentType::TEXT_PLAIN).body(req.txt_body.clone())?
        };
        trace!(
            "[Tardis.MailClient] Sending email:{}, from: {}, to: {}",
            req.subject,
            req.from.as_ref().unwrap_or(&self.default_from.clone()),
            req.to.join(",")
        );
        match self.client.send(email).await {
            Ok(_) => Ok(()),
            Err(error) => Err(TardisError::internal_error(
                &format!("[Tardis.MailClient] Could not send email: {error}"),
                "-1-tardis-mail-error",
            )),
        }
    }

    pub fn send_quiet(module_code: String, req: TardisMailSendReq) -> TardisResult<()> {
        tokio::spawn(async move {
            let client = TardisFuns::mail_by_module_or_default(&module_code);
            match client.send(&req).await {
                Ok(_) => (),
                Err(error) => warn!("{error:?} | send data: {req:?}"),
            }
        });
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TardisMailSendReq {
    pub subject: String,
    pub txt_body: String,
    pub html_body: Option<String>,
    pub to: Vec<String>,
    pub reply_to: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub from: Option<String>,
}

impl From<address::AddressError> for TardisError {
    fn from(error: address::AddressError) -> Self {
        error!("[Tardis.MailClient] AddressError: {}", error.to_string());
        TardisError::wrap(&format!("[Tardis.MailClient] {error:?}"), "-1-tardis-mail-error")
    }
}

impl From<error::Error> for TardisError {
    fn from(error: error::Error) -> Self {
        error!("[Tardis.MailClient] Error: {}", error.to_string());
        TardisError::wrap(&format!("[Tardis.MailClient] {error:?}"), "406-tardis-mail-addr-error")
    }
}

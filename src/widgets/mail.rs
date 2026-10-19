//! The newest messages in an IMAP mailbox, read-only: the mailbox is opened
//! with EXAMINE and headers are fetched with BODY.PEEK, so nothing is marked
//! read. A small client of our own keeps the dependency set to the TLS stack
//! the HTTP client already uses, plus a header parser.

use std::{net::ToSocketAddrs, sync::Arc, time::Duration};

use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use super::FetchResult;
use crate::config::Mail;

const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    uid: u32,
    subject: String,
    from: String,
    address: String,
    /// RFC 3339, from the Date header, else the server's arrival time.
    date: Option<String>,
    unread: bool,
    message_id: Option<String>,
}

pub async fn fetch(config: &Mail) -> FetchResult {
    tokio::time::timeout(TIMEOUT, fetch_inner(config))
        .await
        .map_err(|_| "邮箱：连接超时".to_owned())?
}

async fn fetch_inner(config: &Mail) -> FetchResult {
    let mut client = Client::connect(config).await?;
    client
        .login(&config.username, &config.password)
        .await
        .map_err(|error| match error {
            Error::No(_) => "邮箱：用户名或密码被拒绝".into(),
            other => other.describe(),
        })?;
    let exists = client
        .examine(&config.mailbox)
        .await
        .map_err(|error| error.describe())?;
    let unread = client
        .search_unseen()
        .await
        .map_err(|error| error.describe())?;
    let messages = if exists == 0 {
        Vec::new()
    } else {
        let first = exists.saturating_sub(config.limit as u32 - 1).max(1);
        let mut messages = client
            .fetch_headers(first, exists)
            .await
            .map_err(|error| error.describe())?;
        // Newest first.
        messages.sort_by_key(|message| std::cmp::Reverse(message.uid));
        messages
    };
    client.logout().await;
    Ok(serde_json::json!({
        "messages": messages,
        "total": exists,
        "unread": unread,
        "mailbox": config.mailbox,
        "link": config.link,
    }))
}

// ------------------------------------------------------------------ client

enum Error {
    Io(std::io::Error),
    /// The server answered the command with NO.
    No(String),
    /// The server answered with BAD, or something unparsable.
    Protocol(String),
}

impl Error {
    fn describe(&self) -> String {
        match self {
            Self::Io(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                "邮箱：连接超时".into()
            }
            Self::Io(_) => "邮箱：无法连接".into(),
            Self::No(text) => format!("邮箱：{}", text.trim()),
            Self::Protocol(detail) => {
                tracing::debug!(detail, "imap protocol error");
                "邮箱：服务器响应异常".into()
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

type Stream = Box<dyn Transport>;

trait Transport: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> Transport for T {}

struct Client {
    reader: BufReader<Stream>,
    tag: u32,
}

/// One untagged line of a response, with any literal that followed it.
struct Line {
    text: String,
    literal: Vec<u8>,
}

impl Client {
    async fn connect(config: &Mail) -> Result<Self, String> {
        let address = (config.host.as_str(), config.port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addresses| addresses.next())
            .ok_or_else(|| "邮箱：无法解析服务器地址".to_owned())?;
        let tcp = TcpStream::connect(address)
            .await
            .map_err(|_| "邮箱：无法连接".to_owned())?;
        let stream: Stream = if config.tls {
            let mut roots = tokio_rustls::rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let tls = tokio_rustls::rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            let name = tokio_rustls::rustls::pki_types::ServerName::try_from(config.host.clone())
                .map_err(|_| "邮箱：服务器地址无效".to_owned())?;
            let stream = tokio_rustls::TlsConnector::from(Arc::new(tls))
                .connect(name, tcp)
                .await
                .map_err(|_| "邮箱：TLS 握手失败".to_owned())?;
            Box::new(stream)
        } else {
            Box::new(tcp)
        };
        let mut client = Self {
            reader: BufReader::new(stream),
            tag: 0,
        };
        // The greeting.
        let greeting = client.read_line().await.map_err(|error| error.describe())?;
        if !greeting.text.starts_with("* OK") && !greeting.text.starts_with("* PREAUTH") {
            return Err("邮箱：服务器响应异常".into());
        }
        Ok(client)
    }

    async fn login(&mut self, username: &str, password: &str) -> Result<(), Error> {
        // Both are sent as quoted strings; a quote or backslash inside is escaped.
        let command = format!("LOGIN {} {}", quote(username), quote(password));
        self.command(&command).await.map(|_| ())
    }

    async fn examine(&mut self, mailbox: &str) -> Result<u32, Error> {
        let lines = self.command(&format!("EXAMINE {}", quote(mailbox))).await?;
        lines
            .iter()
            .find_map(|line| {
                let rest = line.text.strip_prefix("* ")?;
                let (count, word) = rest.split_once(' ')?;
                (word.trim() == "EXISTS")
                    .then(|| count.parse().ok())
                    .flatten()
            })
            .ok_or_else(|| Error::Protocol("no EXISTS".into()))
    }

    async fn search_unseen(&mut self) -> Result<u32, Error> {
        let lines = self.command("SEARCH UNSEEN").await?;
        Ok(lines
            .iter()
            .filter_map(|line| line.text.strip_prefix("* SEARCH"))
            .map(|rest| rest.split_whitespace().count() as u32)
            .sum())
    }

    async fn fetch_headers(&mut self, first: u32, last: u32) -> Result<Vec<Message>, Error> {
        let command = format!(
            "FETCH {first}:{last} (UID FLAGS INTERNALDATE BODY.PEEK[HEADER.FIELDS (SUBJECT FROM DATE MESSAGE-ID)])"
        );
        let lines = self.command(&command).await?;
        Ok(lines
            .iter()
            .filter(|line| line.text.starts_with("* ") && line.text.contains(" FETCH ("))
            .filter_map(|line| parse_message(&line.text, &line.literal))
            .collect())
    }

    async fn logout(&mut self) {
        let _ = self.command("LOGOUT").await;
    }

    /// Sends one command and collects the untagged lines until its tagged reply.
    async fn command(&mut self, command: &str) -> Result<Vec<Line>, Error> {
        self.tag += 1;
        let tag = format!("A{:03}", self.tag);
        self.reader
            .get_mut()
            .write_all(format!("{tag} {command}\r\n").as_bytes())
            .await?;
        let mut lines = Vec::new();
        loop {
            let line = self.read_line().await?;
            if let Some(status) = line
                .text
                .strip_prefix(&tag)
                .and_then(|rest| rest.strip_prefix(' '))
            {
                let (word, text) = status.split_once(' ').unwrap_or((status, ""));
                return match word {
                    "OK" => Ok(lines),
                    "NO" => Err(Error::No(text.to_owned())),
                    _ => Err(Error::Protocol(text.to_owned())),
                };
            }
            lines.push(line);
        }
    }

    /// Reads one line; a trailing `{n}` announces a literal of n bytes, which is
    /// read too, followed by the rest of that line (the closing parenthesis).
    async fn read_line(&mut self) -> Result<Line, Error> {
        let mut text = String::new();
        let mut literal = Vec::new();
        loop {
            let mut raw = Vec::new();
            let read = self.reader.read_until(b'\n', &mut raw).await?;
            if read == 0 {
                return Err(Error::Protocol("connection closed".into()));
            }
            let part = String::from_utf8_lossy(&raw).trim_end().to_owned();
            match literal_size(&part) {
                Some(size) => {
                    text.push_str(&part);
                    let mut bytes = vec![0; size];
                    self.reader.read_exact(&mut bytes).await?;
                    literal.extend(bytes);
                }
                None => {
                    text.push_str(&part);
                    return Ok(Line { text, literal });
                }
            }
        }
    }
}

fn literal_size(line: &str) -> Option<usize> {
    let inner = line.strip_suffix('}')?;
    let start = inner.rfind('{')?;
    inner[start + 1..].parse().ok()
}

fn quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// `* 12 FETCH (UID 345 FLAGS (\Seen) INTERNALDATE "17-Oct-2026 10:00:00 +0000" BODY[HEADER.FIELDS (…)] {n}` plus the header bytes.
fn parse_message(text: &str, header: &[u8]) -> Option<Message> {
    let uid = attribute(text, "UID ")?.split(' ').next()?.parse().ok()?;
    let flags = attribute(text, "FLAGS (")
        .and_then(|rest| rest.split(')').next())
        .unwrap_or("");
    let unread = !flags
        .split_whitespace()
        .any(|flag| flag.eq_ignore_ascii_case("\\Seen"));
    let internal = attribute(text, "INTERNALDATE \"")
        .and_then(|rest| rest.split('"').next())
        .and_then(|raw| chrono::DateTime::parse_from_str(raw, "%d-%b-%Y %H:%M:%S %z").ok())
        .map(|date| date.to_rfc3339());
    let (headers, _) = mailparse::parse_headers(header).ok()?;
    let value = |name: &str| {
        headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case(name))
            .map(|h| h.get_value())
    };
    let subject = value("Subject").unwrap_or_default();
    let (from, address) = value("From")
        .and_then(|raw| mailparse::addrparse(&raw).ok())
        .and_then(|list| list.into_inner().into_iter().next())
        .map(|entry| match entry {
            mailparse::MailAddr::Single(info) => (info.display_name.unwrap_or_default(), info.addr),
            mailparse::MailAddr::Group(group) => (group.group_name, String::new()),
        })
        .unwrap_or_default();
    let date = value("Date")
        .and_then(|raw| mailparse::dateparse(&raw).ok())
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .map(|date| date.to_rfc3339())
        .or(internal);
    let message_id = value("Message-ID").map(|id| id.trim().trim_matches(['<', '>']).to_owned());
    Some(Message {
        uid,
        subject: subject.trim().to_owned(),
        from: if from.is_empty() {
            address.clone()
        } else {
            from
        },
        address,
        date,
        unread,
        message_id,
    })
}

fn attribute<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.find(name).map(|at| &text[at + name.len()..])
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_fetch_response() {
        let text = r#"* 12 FETCH (UID 345 FLAGS (\Flagged) INTERNALDATE "17-Oct-2026 10:00:00 +0800" BODY[HEADER.FIELDS (SUBJECT FROM DATE MESSAGE-ID)] {160})"#;
        let header = b"Subject: =?UTF-8?B?5Y+R56Wo5bey5byA5YW3?=\r\nFrom: \"Chao Zhang\" <chao@example.com>\r\nDate: Fri, 17 Oct 2026 09:30:00 +0800\r\nMessage-ID: <abc@example.com>\r\n\r\n";
        let message = parse_message(text, header).unwrap();
        assert_eq!(message.uid, 345);
        assert_eq!(message.subject, "发票已开具");
        assert_eq!(message.from, "Chao Zhang");
        assert_eq!(message.address, "chao@example.com");
        assert!(message.unread);
        assert_eq!(message.message_id.as_deref(), Some("abc@example.com"));
        assert_eq!(message.date.as_deref(), Some("2026-10-17T01:30:00+00:00"));

        let seen = parse_message(
            &text.replace("\\Flagged", "\\Seen \\Flagged"),
            b"From: x@y.z\r\n\r\n",
        )
        .unwrap();
        assert!(!seen.unread);
        assert_eq!(seen.from, "x@y.z");
        assert_eq!(seen.date.as_deref(), Some("2026-10-17T10:00:00+08:00"));
    }

    #[test]
    fn literal_sizes_and_quoting() {
        assert_eq!(literal_size("* 1 FETCH (BODY[HEADER] {42}"), Some(42));
        assert_eq!(literal_size("* 1 FETCH (UID 1)"), None);
        assert_eq!(quote(r#"pa"ss\word"#), r#""pa\"ss\\word""#);
    }
}

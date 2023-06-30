use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

use crate::configuration::EmailBaseUrl;
use crate::domain::SubscriberEmail;

pub struct EmailClient {
    base_url: EmailBaseUrl,
    http_client: Client,
    sender: SubscriberEmail,
    auth_token: Secret<String>,
}

impl EmailClient {
    pub fn new(base_url: EmailBaseUrl, sender: SubscriberEmail, auth_token: Secret<String>) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();

        Self {
            base_url,
            http_client,
            sender,
            auth_token,
        }
    }
    
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str
    ) -> Result<(), reqwest::Error> {
        let url = self.base_url.as_ref()
            .join("/email")
            .unwrap();
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content
        };
        self.http_client
            .post(url)
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}


#[cfg(test)]
mod tests {
    use claims::{assert_ok, assert_err};
    use fake::{Fake, Faker};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use secrecy::Secret;
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};
    use wiremock::matchers::{any, header, header_exists, method, path};

    use crate::configuration::EmailBaseUrl;
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;


    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let mock_base_url = EmailBaseUrl::try_from(mock_server.uri()).expect("Couldn't convert URI.");
        let email_client = EmailClient::new(
            mock_base_url,
            sender,
            Secret::new(Faker.fake())
        );

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let mock_base_url = EmailBaseUrl::try_from(mock_server.uri()).expect("Couldn't convert URI.");
        let email_client = EmailClient::new(
            mock_base_url,
            sender,
            Secret::new(Faker.fake())
        );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let mock_base_url = EmailBaseUrl::try_from(mock_server.uri()).expect("Couldn't convert URI.");
        let email_client = EmailClient::new(
            mock_base_url,
            sender,
            Secret::new(Faker.fake())
        );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;
        
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let mock_base_url = EmailBaseUrl::try_from(mock_server.uri()).expect("Couldn't convert URI.");
        let email_client = EmailClient::new(
            mock_base_url,
            sender,
            Secret::new(Faker.fake())
        );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        let response = ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
        
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }
}

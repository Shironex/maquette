//! What went wrong on the wire, in words: the whole error source chain, so
//! the queue shows the real cause ("tcp connect error: No route to host"),
//! with every link taken out because result links carry signed queries.

use super::ApiError;

/// hyper's wrapper around the real cause, which says nothing on its own.
const WRAPPER: &str = "client error (";

/// Replaces anything that looks like a link with `<link>`.
fn scrub(text: &str) -> String {
    text.split(' ')
        .map(|word| if word.contains("://") { "<link>" } else { word })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The causes under `error`, outermost first, without wrappers or repeats.
fn causes(error: &dyn std::error::Error) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut source = error.source();
    while let Some(inner) = source {
        let text = scrub(inner.to_string().trim());
        if !text.is_empty() && !text.starts_with(WRAPPER) && parts.last() != Some(&text) {
            parts.push(text);
        }
        source = inner.source();
    }
    parts
}

/// Sorts a failed request into "never sent" and "may have arrived".
///
/// A connect error (DNS, TCP, TLS or a connect timeout) comes from opening
/// the connection, before any byte of the request is written, so the
/// provider cannot have seen it. Anything later is unclear: the request
/// may have arrived even when the answer never came back.
pub(super) fn classify(error: reqwest::Error) -> ApiError {
    let error = error.without_url();
    let chain = causes(&error).join(": ");
    if error.is_connect() {
        return ApiError::Unreachable(if chain.is_empty() {
            "the connection failed".to_string()
        } else {
            chain
        });
    }
    let head = if error.is_timeout() {
        "the request timed out".to_string()
    } else {
        scrub(&error.to_string())
    };
    ApiError::Network(if chain.is_empty() {
        format!("{head}.")
    } else {
        format!("{head}: {chain}.")
    })
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::*;

    #[derive(Debug)]
    struct Layer(&'static str, Option<Box<Layer>>);

    impl fmt::Display for Layer {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for Layer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.1
                .as_deref()
                .map(|l| l as &(dyn std::error::Error + 'static))
        }
    }

    fn chain(texts: &[&'static str]) -> Layer {
        texts
            .iter()
            .rev()
            .fold(None, |inner, text| Some(Layer(text, inner.map(Box::new))))
            .expect("at least one layer")
    }

    #[test]
    fn the_chain_keeps_the_cause_and_drops_wrappers_repeats_and_links() {
        let error = chain(&[
            "error sending request",
            "client error (Connect)",
            "tcp connect error",
            "tcp connect error",
            "No route to host (os error 65) at https://x.test/a?sig=secret",
        ]);
        assert_eq!(
            causes(&error).join(": "),
            "tcp connect error: No route to host (os error 65) at <link>"
        );
    }

    #[test]
    fn a_refused_connection_is_unreachable_and_names_the_cause() {
        crate::http::ensure_crypto();
        let error = reqwest::blocking::Client::new()
            .get("http://127.0.0.1:1/never?token=secret")
            .send()
            .expect_err("nothing listens on port 1");
        match classify(error) {
            ApiError::Unreachable(detail) => {
                assert!(detail.contains("tcp connect error"), "{detail}");
                assert!(!detail.contains("secret"), "{detail}");
            }
            other => panic!("expected Unreachable, got {other:?}"),
        }
    }
}

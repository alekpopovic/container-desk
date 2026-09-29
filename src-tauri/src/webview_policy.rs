//! Only the bundled document may occupy the main window. CSP alone does not fence navigation.
use tauri::Url;

pub fn navigation_allowed(url: &Url) -> bool {
    permitted(url, cfg!(debug_assertions))
}

fn permitted(url: &Url, development: bool) -> bool {
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || !matches!(url.path(), "" | "/" | "/index.html")
    {
        return false;
    }
    (url.scheme() == "tauri" && url.host_str() == Some("localhost") && url.port().is_none())
        || (development
            && url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(1420))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_local_document_and_hash_routes_are_admitted_in_release() {
        for url in [
            "tauri://localhost",
            "tauri://localhost/",
            "tauri://localhost/index.html#/settings",
        ] {
            assert!(permitted(&url.parse().unwrap(), false), "{url}");
        }
        for url in [
            "https://example.com/",
            "http://127.0.0.1:1420/",
            "tauri://localhost.evil/",
            "tauri://evil@localhost/",
            "tauri://localhost:80/",
            "tauri://localhost/?next=https://example.com",
            "tauri://localhost/asset.html",
            "tauri://localhost/%2e%2e/etc/passwd",
            "file:///etc/hosts",
            "data:text/html,<script>alert(1)</script>",
            "javascript:alert(1)",
            "blob:tauri://localhost/example",
            "about:blank",
        ] {
            assert!(!permitted(&url.parse().unwrap(), false), "{url}");
        }
    }
    #[test]
    fn development_adds_only_the_pinned_loopback_origin() {
        assert!(permitted(
            &"http://127.0.0.1:1420/#/hosts".parse().unwrap(),
            true
        ));
        for url in [
            "http://127.0.0.1:1421/",
            "http://localhost:1420/",
            "http://127.0.0.1:1420.evil/",
            "http://127.0.0.1:1420/other.html",
            "http://user@127.0.0.1:1420/",
        ] {
            assert!(
                !url.parse::<Url>().is_ok_and(|url| permitted(&url, true)),
                "{url}"
            );
        }
    }
}

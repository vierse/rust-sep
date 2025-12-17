use url_shorten::api::UrlError;

#[test]
fn scheme() {
    let wrong_scheme_urls = [
        "ftp://user:password@hostname.com/txt.txt",
        "file:///home/user/.bashrc",
        "ssh://login@server.com:12345/repository.git",
        "unix:/run/foo.socket",
    ];

    let wrong_scheme_results = wrong_scheme_urls.map(url_shorten::api::validate_url);

    assert!(
        wrong_scheme_results
            .into_iter()
            .all(|res| res == Err(UrlError::WrongScheme))
    );

    let right_scheme_urls = [
        "http://user:password@hostname.com/txt.txt",
        "https:///home/user/.bashrc",
        "http://login@server.com:12345/repository.git",
        "https:/run/foo.socket",
    ];

    let right_scheme_results = right_scheme_urls.map(url_shorten::api::validate_url);

    assert!(
        right_scheme_results
            .into_iter()
            .all(|res| res != Err(UrlError::WrongScheme))
    );
}

#[test]
fn host() {
    let wrong_host_urls = [
        "http://user:password@localhost/txt.txt",
        "https://user:password@127.0.0.1/txt.txt",
    ];

    let wrong_host_results = wrong_host_urls.map(url_shorten::api::validate_url);

    assert!(
        wrong_host_results
            .into_iter()
            .all(|res| res == Err(UrlError::LocalHost))
    );
}

#[test]
fn val() {
    let valid_urls = [
        "http://user:password@hostname.com/txt.txt",
        "https:///home/user/.bashrc",
        "http://login@server.com:12345/repository.git",
        "https:/run/foo.socket",
    ];

    let valid_results = valid_urls.map(url_shorten::api::validate_url);

    assert!(valid_results.iter().all(Result::is_ok));
}

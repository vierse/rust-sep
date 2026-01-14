mod redirect;
mod shorten;

pub use redirect::redirect;
pub use shorten::recently_added_links;
pub use shorten::shorten;

pub use shorten::ShortenResponse;

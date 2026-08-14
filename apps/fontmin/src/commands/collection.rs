use miette::{Result, miette};

pub fn select_collection_face(bytes: Vec<u8>, font_number: Option<usize>) -> Result<Vec<u8>> {
    let is_collection = bytes.starts_with(b"ttcf");

    match (is_collection, font_number) {
        (true, Some(font_number)) => {
            fontmin::extract_collection_face(&bytes, font_number).map_err(Into::into)
        }
        (true, None) => Err(miette!(
            "TTC/OTC input requires --font-number with a zero-based face index"
        )),
        (false, Some(_)) => Err(miette!("--font-number requires TTC/OTC input")),
        (false, None) => Ok(bytes),
    }
}

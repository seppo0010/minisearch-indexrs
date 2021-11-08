use failure::Fail;

#[derive(Debug, Fail)]
pub enum MinisearchIndexrsError {
    #[fail(display = "item to index does not have an id field")]
    MissingId,
}

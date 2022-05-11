use sea_orm_codegen::{EntityTransformer, EntityWriter, OutputFile, WithSerde};
use sea_schema::sea_query::table::TableCreateStatement;
use std::{fs, io::Result, path::Path};

pub fn generate_entities(dir: &Path, table_create_stmts: Vec<TableCreateStatement>) -> Result<()> {
    let entity_writer: EntityWriter = EntityTransformer::transform(table_create_stmts).unwrap();

    let writer_output = entity_writer.generate(true, WithSerde::None);

    for OutputFile { name, content } in writer_output.files.iter() {
        let file_path = dir.join(name);
        fs::write(file_path, content.as_bytes())?;
    }

    Ok(())
}

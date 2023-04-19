use spirv_builder::{MetadataPrintout, SpirvBuilder, SpirvMetadata};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    SpirvBuilder::new("shader", "spirv-unknown-spv1.5")
        .spirv_metadata(SpirvMetadata::NameVariables)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}

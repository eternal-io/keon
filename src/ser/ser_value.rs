use super::*;

impl Seriable for Value {
    fn seria_via<W: Write>(&self, ser: &mut Serria<W>) -> SeriaResult {
        todo!()
    }
}

impl<W: Write> Serria<W> {
    fn write_seq(&mut self, seq: &Values) -> SeriaResult {
        let mut ent: SerriaEntry<'_, W> = todo!();

        for val in seq {
            ent.serialize(*val)?;
        }

        ent.leave()?;

        Ok(())
    }
}

use crate::types::{PTypes, PercentilesCalculator, PercentilesCalculatorOptions, PercentilesError};

pub struct CalculatorImpl;

impl<P: PTypes> PercentilesCalculator<P> for CalculatorImpl {
    fn new(options: PercentilesCalculatorOptions<P>) -> Self {
        CalculatorImpl
    }

    fn calculate(&mut self) -> Result<(), PercentilesError<P>> {
        todo!()
    }
}

pub type Code = Vec<Instruction>;

#[derive(Debug)]
pub enum Instruction {
    Regexp(Regexp),
    Vars(VarsYaml),
}

#[derive(Debug)]
pub struct Regexp {
    //
}

#[derive(Debug)]
pub struct VarsYaml {
    //
}

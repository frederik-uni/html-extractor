use crate::{
    CompileError, FunctionDefinition, Program, Requirements, SourceText, Statement, TestBlock,
    parse, validation::validate,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledProgram {
    source: SourceText,
    program: Program,
    requirements: Requirements,
}

impl CompiledProgram {
    #[must_use]
    pub const fn requirements(&self) -> &Requirements {
        &self.requirements
    }

    #[must_use]
    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    #[must_use]
    pub fn statements(&self) -> &[Statement] {
        self.program.statements()
    }

    #[must_use]
    pub fn functions(&self) -> &[FunctionDefinition] {
        self.program.functions()
    }

    #[must_use]
    pub fn tests(&self) -> &[TestBlock] {
        self.program.tests()
    }
}

pub trait ProgramView {
    fn requirements(&self) -> &Requirements;
    fn source(&self) -> &str;
    fn statements(&self) -> &[Statement];
    fn functions(&self) -> &[FunctionDefinition];
    fn tests(&self) -> &[TestBlock];
}

impl ProgramView for CompiledProgram {
    fn requirements(&self) -> &Requirements {
        self.requirements()
    }

    fn source(&self) -> &str {
        self.source()
    }

    fn statements(&self) -> &[Statement] {
        self.statements()
    }

    fn functions(&self) -> &[FunctionDefinition] {
        self.functions()
    }

    fn tests(&self) -> &[TestBlock] {
        self.tests()
    }
}

pub fn compile(source: &str) -> Result<CompiledProgram, CompileError> {
    let mut program = parse(source)?;
    let requirements = validate(&mut program)?;
    Ok(CompiledProgram {
        source: SourceText::new(source),
        program,
        requirements,
    })
}

#[derive(Deserialize, Serialize)]
struct EncodedProgram {
    program: Program,
    requirements: Requirements,
}

#[doc(hidden)]
pub fn __encode_compiled(program: &CompiledProgram) -> Vec<u8> {
    bincode::serialize(&EncodedProgram {
        program: program.program.clone(),
        requirements: program.requirements.clone(),
    })
    .expect("compiled program must be serializable")
}

#[doc(hidden)]
#[must_use]
pub fn __decode_compiled(source: &str, encoded: &[u8]) -> CompiledProgram {
    let payload: EncodedProgram =
        bincode::deserialize(encoded).expect("macro emitted an invalid compiled program");
    CompiledProgram {
        source: SourceText::new(source),
        program: payload.program,
        requirements: payload.requirements,
    }
}

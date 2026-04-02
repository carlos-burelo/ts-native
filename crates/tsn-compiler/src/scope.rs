#[derive(Clone, Debug)]
pub struct Local {
    pub name: String,
    pub depth: i32,
    pub is_captured: bool,
}

#[derive(Clone, Debug)]
pub struct Upvalue {
    pub is_local: bool,
    pub index: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Scope {
    pub locals: Vec<Local>,
    pub upvalues: Vec<Upvalue>,
    pub depth: i32,
}

impl Scope {
    pub fn new() -> Self {
        Scope::default()
    }

    pub fn push_block(&mut self) {
        self.depth += 1;
    }

    pub fn pop_block(&mut self) -> (usize, Vec<bool>) {
        let count = self
            .locals
            .iter()
            .rev()
            .take_while(|l| l.depth == self.depth)
            .count();
        let captured = self.locals[self.locals.len() - count..]
            .iter()
            .map(|l| l.is_captured)
            .rev()
            .collect();
        let len = self.locals.len();
        self.locals.truncate(len - count);
        self.depth -= 1;
        (count, captured)
    }

    pub fn declare_local(&mut self, name: impl Into<String>) -> u16 {
        let slot = self.locals.len() as u16;
        self.locals.push(Local {
            name: name.into(),
            depth: self.depth,
            is_captured: false,
        });
        slot
    }

    pub fn resolve_local(&self, name: &str) -> Option<u16> {
        self.locals
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.name == name)
            .map(|(i, _)| i as u16)
    }

    pub fn add_upvalue(&mut self, is_local: bool, index: u8) -> u8 {
        for (i, uv) in self.upvalues.iter().enumerate() {
            if uv.is_local == is_local && uv.index == index {
                return i as u8;
            }
        }
        let idx = self.upvalues.len() as u8;
        self.upvalues.push(Upvalue { is_local, index });
        idx
    }

    pub fn local_count(&self) -> usize {
        self.locals.len()
    }
}

#[derive(Clone, Debug, Default)]
pub struct LoopContext {
    pub break_patches: Vec<usize>,
    pub continue_patches: Vec<usize>,
    pub locals_before_hidden: usize,
    pub locals_at_body_start: usize,
}

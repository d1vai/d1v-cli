use std::io;

use super::{Render, RenderContext};

#[derive(Default)]
pub struct Stack {
    children: Vec<Box<dyn Render>>,
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(mut self, child: impl Render + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    pub fn children<E>(mut self, children: impl IntoIterator<Item = E>) -> Self
    where
        E: Render + 'static,
    {
        self.children.extend(
            children
                .into_iter()
                .map(|child| Box::new(child) as Box<dyn Render>),
        );
        self
    }
}

impl Render for Stack {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        for child in &self.children {
            child.render(ctx)?;
        }

        Ok(())
    }
}

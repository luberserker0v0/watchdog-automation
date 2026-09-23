#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum StreamOwner {
    #[default]
    None,
    Manual,
    Pipeline {
        run_id: String,
    },
}

impl StreamOwner {
    pub(super) fn acquire_manual(&mut self) -> Result<(), String> {
        if let Self::Pipeline { run_id } = self {
            return Err(format!(
                "Capture is owned by pipeline '{run_id}'; stop it before using this tool"
            ));
        }
        *self = Self::Manual;
        Ok(())
    }

    pub(super) fn acquire_pipeline(&mut self, run_id: &str) -> Result<(), String> {
        match self {
            Self::Manual => {
                return Err(
                    "Capture is owned by manual streaming; stop it before starting a pipeline"
                        .to_string(),
                )
            }
            Self::Pipeline { run_id } => {
                return Err(format!(
                "Capture is owned by pipeline '{run_id}'; stop it before starting another pipeline"
            ))
            }
            Self::None => {}
        }
        *self = Self::Pipeline {
            run_id: run_id.to_string(),
        };
        Ok(())
    }

    pub(super) fn reject_pipeline(&self) -> Result<(), String> {
        if let Self::Pipeline { run_id } = self {
            return Err(format!(
                "Capture is owned by pipeline '{run_id}'; stop it before using this tool"
            ));
        }
        Ok(())
    }

    pub(super) fn release_manual(&mut self) {
        if matches!(self, Self::Manual) {
            *self = Self::None;
        }
    }

    pub(super) fn release_pipeline(&mut self, run_id: &str) {
        if matches!(self, Self::Pipeline { run_id: owner_id } if owner_id == run_id) {
            *self = Self::None;
        }
    }

    pub(super) fn status(&self) -> (&'static str, Option<String>) {
        match self {
            Self::None => ("none", None),
            Self::Manual => ("manual", None),
            Self::Pipeline { run_id } => ("pipeline", Some(run_id.clone())),
        }
    }
}

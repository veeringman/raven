//! Registry of capabilities visible in the current environment.
//!
//! A registered capability is discoverable. It is not authorized.

use raven_core::Capability;

#[derive(Clone, Debug, Default)]
pub struct Registry {
    capabilities: Vec<Capability>,
}

impl Registry {
    pub fn register(&mut self, capability: Capability) {
        if let Some(existing) = self
            .capabilities
            .iter_mut()
            .find(|item| item.id == capability.id)
        {
            *existing = capability;
        } else {
            self.capabilities.push(capability);
        }
    }

    pub fn revoke(&mut self, id: &str) {
        self.capabilities.retain(|item| item.id != id);
    }

    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|item| item.id == id)
    }

    /// Capabilities the agent may consider. Forbidden entries stay hidden.
    pub fn available(&self) -> Vec<Capability> {
        self.capabilities
            .iter()
            .filter(|item| item.available && !item.forbidden)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raven_core::{Capability, RiskLevel};

    #[test]
    fn revoke_and_forbidden_drop_out_of_discovery() {
        let mut registry = Registry::default();
        registry.register(Capability::new(
            "calendar",
            "Read the calendar",
            RiskLevel::L0,
        ));
        let mut camera = Capability::new("camera", "Capture a photo", RiskLevel::L1);
        camera.forbidden = true;
        registry.register(camera);
        registry.register(Capability::new("files", "Read a file", RiskLevel::L1));
        registry.revoke("files");

        let ids: Vec<_> = registry
            .available()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(ids, vec!["calendar".to_string()]);
    }
}

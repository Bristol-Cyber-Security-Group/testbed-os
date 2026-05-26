use crate::state::schema::guest::{StateGuestType, StateTestbedGuest};
use crate::state::state_evaluation::EvaluateState;
use async_trait::async_trait;

#[async_trait]
impl EvaluateState for StateTestbedGuest {
    async fn get_check_command(&self, project_name: String) -> String {
        match self.guest_type.guest_type {
            StateGuestType::Libvirt(_) => {
                format!(
                    "virsh domstate \"{}-{}\" 2>/dev/null || echo \"does_not_exist\"",
                    project_name,
                    self.guest_type.name,
                )
            }
            StateGuestType::Docker(_) => {
                format!(
                    "docker inspect -f '{{{{.State.Status}}}}' \"{}-{}\" 2>/dev/null || echo \"does_not_exist\"",
                    project_name,
                    self.guest_type.name,
                )
            }
            StateGuestType::Android(_) => {
                format!(
                    "emulator -list-avds 2>/dev/null | grep -Fxq \"{0}-{1}\" && (pgrep -f \"emulator.*-name {0}-{1}\" >/dev/null && echo \"up\" || echo \"shut off\") || echo \"does_not_exist\"",
                    project_name,
                    self.guest_type.name,
                )
            }
        }
    }
}


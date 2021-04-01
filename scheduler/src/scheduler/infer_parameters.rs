use super::{Scheduler, SchedulerInput};

impl Scheduler {

    pub(super) fn infer_parameters<I: SchedulerInput>(&mut self, input: &I) {
        println!("{:?}", self.active_passes);

        for scheduler in self.active_passes.iter() {
            let mut pass = &mut self.passes[*scheduler];

            let mut extent = None;
            for attachment_data in pass.attachment_data.iter() {
                let resource_id = attachment_data.resource;
                let image_data = input.resource_data(resource_id).image();
                if let Some(kind) = image_data.kind {
                    let new_extent = kind.extent();
                    if let Some(extent) = extent {
                        assert!(extent == new_extent);
                    } else {
                        extent = Some(new_extent);
                    }
                }
            }

            pass.extent = Some(extent.unwrap());
        }

    }

}

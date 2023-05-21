#[macro_export]
macro_rules! create_stepper_hook {
    ($name: ident, $init: literal, $structure: literal, $count: literal) => {
        paste! {
            static_detour! { static [<$name:upper _HOOK>]: fn(); }
        }

        repeated!(for z in [0;$count] {
            paste! {
                static_detour! { static [<$name:upper _%%z%% _HOOK>]: fn(usize, usize); }
            }
        });

        paste! { static mut [<NAMES_ $name:upper>] : Option<Vec<String>> = None; }

        unsafe fn $name() {
            paste!{ [<$name:upper _HOOK>] }.initialize(mem::transmute($init as usize), move || {
                paste!{ [<$name:upper _HOOK>] }.call();

                let mut names = Vec::<String>::new();
                repeated!(for z in [0;$count] {
                    let step = &*((($structure as usize) + 0x10 * %%z%%) as *const StepperStep);

                    if ((*step).function_ptr != 0x0) {
                        let step_name = (*step).name_ptr.to_string().unwrap();
                        names.push(step_name.clone());
                        paste! { [<$name:upper _%%z%% _HOOK>] }
                            .initialize(mem::transmute(step.function_ptr), move |step: usize, param_2: usize| {
                                let name = paste! { [<NAMES_ $name:upper>] }.as_ref().unwrap()[%%z%%].as_str();
                                // debug!("Hook {} was invoked", name);
                                let span = span!(Level::INFO, "Step", step = name);
                                let _enter = span.enter();

                                paste! { [<$name:upper _%%z%% _HOOK>] }.call(step, param_2);
                            });

                        match paste! { [<$name:upper _%%z%% _HOOK>] }.enable() {
                            Err(e) => warn!("Could not hook {} (#{} - {:#x})", step_name, %%z%%, step.function_ptr),
                            _ => {},
                        };
                    } else {
                        warn!("Found empty step at step {}", %%z%%);
                    }
                });

                paste! { [<NAMES_ $name:upper>] = Some(names); }
            }).unwrap();

            paste!{ [<$name:upper _HOOK>] }.enable().unwrap();
        }
    };
}

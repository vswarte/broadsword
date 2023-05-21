use std::mem;

use log::*;
use paste::paste;
use repeated::repeated;
use windows::core::PWSTR;
use detour::static_detour;
use tracing::{span, Level};

use crate::create_stepper_hook;

create_stepper_hook!(move_map_step, 0x1400a3cf0, 0x143ce7590, 19);
create_stepper_hook!(system_step, 0x1400b12a0, 0x143cfc3c0, 20);
create_stepper_hook!(network_flow_step, 0x1400ab1f0, 0x143cf0f10, 4);
create_stepper_hook!(regulation_step, 0x1400b2750, 0x143cfd950, 7);
create_stepper_hook!(camera_step, 0x140093770, 0x143cd9840, 3);
create_stepper_hook!(chara_select, 0x1400a3650, 0x143ce70c8, 1);
create_stepper_hook!(draw, 0x1400a5030, 0x143ce86b0, 3);
create_stepper_hook!(event_flag_res, 0x14009bbd0, 0x143cdf2c0, 3);
create_stepper_hook!(fd4_mowwisebank_res_cap_task, 0x1400b0790, 0x143cfb1c0, 3);
create_stepper_hook!(fd4_location, 0x1400a5820, 0x143ce9220, 3);
create_stepper_hook!(file, 0x14007e360, 0x143cd20b0, 3);
create_stepper_hook!(event_world_area_time, 0x14009c8c0, 0x143cdfd60, 7);
create_stepper_hook!(scaleform, 0x1400ae5b0, 0x143cf9ed0, 18);
create_stepper_hook!(emk_system_update, 0x140097080, 0x143cdea38, 2);
create_stepper_hook!(dist_view_manager, 0x140096910, 0x143cde3d0, 7);

pub(crate) unsafe fn hook() {
    move_map_step();
    system_step();
    network_flow_step();
    regulation_step();
    camera_step();
    chara_select();
    draw();
    event_flag_res();
    fd4_mowwisebank_res_cap_task();
    fd4_location();
    file();
    event_world_area_time();
    scaleform();
    emk_system_update();
    dist_view_manager();
}

#[repr(C)]
#[derive(Debug, Clone)]
struct StepperStep {
    pub function_ptr: usize,
    pub name_ptr: PWSTR,
}

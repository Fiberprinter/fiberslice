use std::io::Write;

use glam::vec2;
use navigator::Navigator;

use super::{settings::Settings, Command, RetractionType};

pub mod navigator;

#[derive(Debug)]
pub struct SlicedGCode {
    pub gcode: String,
    pub line_breaks: Vec<usize>,
    pub navigator: Navigator,
}

impl SlicedGCode {
    pub fn new(gcode: String, navigator: Navigator) -> Self {
        let line_breaks = gcode
            .char_indices()
            .filter_map(|(i, c)| if c == '\n' { Some(i) } else { None })
            .collect();

        Self {
            gcode,
            line_breaks,
            navigator,
        }
    }
}

pub trait WriteGCode: Write {
    fn line_count(&self) -> usize;
}

pub mod mem {
    use std::io::Write;

    use super::{navigator::Navigator, SlicedGCode, WriteGCode};

    pub struct GCodeMemoryWriter {
        buffer: String,
        line_count: usize,
    }

    impl GCodeMemoryWriter {
        pub fn new() -> Self {
            Self {
                buffer: String::new(),
                line_count: 0,
            }
        }

        pub fn finish(self, navigator: Navigator) -> SlicedGCode {
            SlicedGCode::new(self.buffer, navigator)
        }
    }

    impl WriteGCode for GCodeMemoryWriter {
        fn line_count(&self) -> usize {
            self.line_count
        }
    }

    impl Write for GCodeMemoryWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let s = std::str::from_utf8(buf).unwrap();

            self.line_count += s.chars().filter(|c| *c == '\n').count();

            self.buffer.push_str(s);

            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

pub struct GCodeFileWriter<T: Write> {
    writer: T,
    line_count: usize,
}

impl<T: Write> GCodeFileWriter<T> {
    pub fn new(writer: T) -> Self {
        Self {
            writer,
            line_count: 0,
        }
    }
}

impl<T: Write> WriteGCode for GCodeFileWriter<T> {
    fn line_count(&self) -> usize {
        self.line_count
    }
}

impl<T: Write> Write for GCodeFileWriter<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = std::str::from_utf8(buf).unwrap();

        self.line_count += s.chars().filter(|c| *c == '\n').count();

        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub fn write_gcode(
    cmds: &[Command],
    settings: &Settings,
    writer: &mut dyn WriteGCode,
) -> Result<Navigator, Box<dyn std::error::Error>> {
    let mut current_z = 0.0;
    let mut layer_count = 0;
    let mut current_object = None;

    let mut navigator = Navigator::new(cmds.len());

    let start = convert_instructions(
        settings.starting_instructions.clone(),
        current_z,
        layer_count,
        None,
        current_object,
        settings,
    );

    writeln!(
        writer,
        "M201 X{:.1} Y{:.1} Z{:.1} E{:.1}; sets maximum accelerations, mm/sec^2",
        settings.max_acceleration_x,
        settings.max_acceleration_y,
        settings.max_acceleration_z,
        settings.max_acceleration_e
    )?;
    writeln!(
        writer,
        "M203 X{:.1} Y{:.1} Z{:.1} E{:.1}; ; sets maximum feedrates, mm/sec",
        settings.maximum_feedrate_x,
        settings.maximum_feedrate_y,
        settings.maximum_feedrate_z,
        settings.maximum_feedrate_e
    )?;
    writeln!(writer, "M204 P{:.1} R{:.1} T{:.1}; sets acceleration (P, T) and retract acceleration (R), mm/sec^2", settings.max_acceleration_extruding, settings.max_acceleration_retracting, settings.max_acceleration_travel)?;
    writeln!(
        writer,
        "M205 X{:.1} Y{:.1} Z{:.1} E{:.1}; sets the jerk limits, mm/sec",
        settings.max_jerk_x, settings.max_jerk_y, settings.max_jerk_z, settings.max_jerk_e
    )?;
    writeln!(
        writer,
        "M205 S{:.1} T{:.1} ; sets the minimum extruding and travel feed rate, mm/sec",
        settings.minimum_feedrate_print, settings.minimum_feedrate_travel
    )?;
    writeln!(writer, "{}", start)?;
    writeln!(writer, "G21 ; set units to millimeters")?;
    writeln!(writer, "G90 ; use absolute Coords")?;
    writeln!(writer, "M83 ; use relative distances for extrusion")?;

    for cmd in cmds {
        match cmd {
            Command::MoveTo { end, .. } => writeln!(writer, "G1 X{:.5} Y{:.5}", end.x, end.y)?,
            Command::MoveAndExtrude {
                id,
                start,
                end,
                width,
                thickness,

                #[cfg(debug_assertions)]
                debug,
                ..
            } => {
                navigator.record_trace(id.expect("Id's not eval yet!"), writer.line_count());

                let x_diff = end.x - start.x;
                let y_diff = end.y - start.y;
                let length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();

                //let extrusion_width = width + (thickness * (1.0 - std::f32::consts::FRAC_PI_4));

                let extrusion_volume = (((width - thickness) * thickness)
                    + (std::f32::consts::PI * (thickness / 2.0) * (thickness / 2.0)))
                    * length;
                /*let extrusion_volume = width*thickness*length;*/

                let filament_area = (std::f32::consts::PI
                    * settings.filament.diameter
                    * settings.filament.diameter)
                    / 4.0;
                let extrude = extrusion_volume / filament_area;

                #[cfg(debug_assertions)]
                writeln!(
                    writer,
                    "G1 X{:.5} Y{:.5} E{:.5} ;{}",
                    end.x, end.y, extrude, debug
                )?;

                #[cfg(not(debug_assertions))]
                writeln!(writer, "G1 X{:.5} Y{:.5} E{:.5}", end.x, end.y, extrude)?;
            }
            Command::MoveAndExtrudeFiber {
                id,
                start,
                end,
                thickness,
                width,

                #[cfg(debug_assertions)]
                debug,
                ..
            } => {
                navigator.record_trace(id.expect("Id's not eval yet!"), writer.line_count());

                let x_diff = end.x - start.x;
                let y_diff = end.y - start.y;
                let length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();

                //let extrusion_width = width + (thickness * (1.0 - std::f32::consts::FRAC_PI_4));

                let extrusion_volume = (((width - thickness) * thickness)
                    + (std::f32::consts::PI * (thickness / 2.0) * (thickness / 2.0)))
                    * length;
                /*let extrusion_volume = width*thickness*length;*/

                let filament_area = (std::f32::consts::PI
                    * settings.filament.diameter
                    * settings.filament.diameter)
                    / 4.0;
                let extrude = extrusion_volume / filament_area;

                #[cfg(debug_assertions)]
                writeln!(
                    writer,
                    "G1 X{:.5} Y{:.5} E{:.5} D{:.5} ;{}",
                    end.x, end.y, extrude, extrude, debug
                )?;

                #[cfg(not(debug_assertions))]
                writeln!(
                    writer,
                    "G1 X{:.5} Y{:.5} E{:.5} D{:.5}",
                    end.x, end.y, extrude, extrude
                )?;
            }
            Command::MoveAndExtrudeFiberAndCut {
                id,
                start,
                end,
                thickness,
                width,

                cut_pos,

                #[cfg(debug_assertions)]
                debug,
            } => {
                navigator.record_trace(id.expect("Id's not eval yet!"), writer.line_count());

                let (start, end) = (vec2(start.x, start.y), vec2(end.x, end.y));

                // direction from end to start
                let direction = start - end;
                let length = direction.length();
                let direction = direction.normalize();

                let cut_pos = end + direction * cut_pos;

                let lenght_before_cut = (cut_pos - start).length();
                let length_after_cut = length - lenght_before_cut;

                //let extrusion_width = width + (thickness * (1.0 - std::f32::consts::FRAC_PI_4));
                let extrude_fn = |length: f32| {
                    let extrusion_volume = (((width - thickness) * thickness)
                        + (std::f32::consts::PI * (thickness / 2.0) * (thickness / 2.0)))
                        * length;
                    /*let extrusion_volume = width*thickness*length;*/

                    let filament_area = (std::f32::consts::PI
                        * settings.filament.diameter
                        * settings.filament.diameter)
                        / 4.0;
                    extrusion_volume / filament_area
                };

                let extrude_before_cut = extrude_fn(lenght_before_cut);

                #[cfg(debug_assertions)]
                writeln!(
                    writer,
                    "G1 X{:.5} Y{:.5} E{:.5} D{:.5} ;{}",
                    cut_pos.x, cut_pos.y, extrude_before_cut, extrude_before_cut, debug
                )?;

                #[cfg(not(debug_assertions))]
                writeln!(
                    writer,
                    "G1 X{:.5} Y{:.5} E{:.5} D{:.5}",
                    cut_pos.x, cut_pos.y, extrude_before_cut, extrude_before_cut
                )?;

                // cut
                writeln!(writer, "M300; cut fiber")?;

                let extrude_after_cut = extrude_fn(length_after_cut);

                #[cfg(debug_assertions)]
                writeln!(
                    writer,
                    "G1 X{:.5} Y{:.5} E{:.5} D{:.5} ;{}",
                    end.x, end.y, extrude_after_cut, extrude_after_cut, debug
                )?;

                #[cfg(not(debug_assertions))]
                writeln!(
                    writer,
                    "G1 X{:.5} Y{:.5} E{:.5} D{:.5}",
                    end.x, end.y, extrude_after_cut, extrude_after_cut
                )?;
            }
            Command::SetState { new_state } => {
                match &new_state.retract {
                    RetractionType::NoRetract => {
                        if let Some(speed) = new_state.movement_speed {
                            writeln!(writer, "G1 F{:.5}", speed * 60.0)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(writer, "M204 S{:.1}", accel)?;
                        }
                    }
                    RetractionType::Retract => {
                        //retract
                        if let Some(speed) = new_state.movement_speed {
                            writeln!(writer, "G1 F{:.5}", speed * 60.0)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(writer, "M204 S{:.1}", accel)?;
                        }

                        writeln!(
                            writer,
                            "G1 E{:.5} F{:.5}; Retract",
                            -settings.retract_length,
                            60.0 * settings.retract_speed,
                        )?;

                        writeln!(
                            writer,
                            "G1 Z{:.5} F{:.5}; z Lift",
                            current_z + settings.retract_lift_z,
                            60.0 * settings.speed.travel,
                        )?;
                    }
                    RetractionType::Unretract => {
                        //unretract
                        writeln!(writer, "G1 Z{:.5}; z unlift", current_z,)?;
                        writeln!(
                            writer,
                            "G1 E{:.5} F{:.5}; Unretract",
                            settings.retract_length,
                            60.0 * settings.retract_speed,
                        )?;

                        if let Some(speed) = new_state.movement_speed {
                            writeln!(writer, "G1 F{:.5}", speed * 60.0)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(writer, "M204 S{:.1}", accel)?;
                        }
                    }
                    RetractionType::MoveRetract(moves) => {
                        if let Some(speed) = new_state.movement_speed {
                            writeln!(writer, "G1 F{:.5}", speed * 60.0)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(writer, "M204 S{:.1}", accel)?;
                        }

                        for (retract_amount, end) in moves {
                            writeln!(
                                writer,
                                "G1 X{:.5} Y{:.5} E{:.5}; Retract with move",
                                end.x, end.y, -retract_amount
                            )?;
                        }

                        writeln!(
                            writer,
                            "G1 Z{:.5} F{:.5}; z Lift",
                            current_z + settings.retract_lift_z,
                            60.0 * settings.speed.travel,
                        )?;
                    }
                }

                if let Some(ext_temp) = new_state.extruder_temp {
                    writeln!(writer, "M104 S{:.1} ; set extruder temp", ext_temp)?;
                }
                if let Some(bed_temp) = new_state.bed_temp {
                    writeln!(writer, "M140 S{:.1} ; set bed temp", bed_temp)?;
                }
                if let Some(fan_speed) = new_state.fan_speed {
                    writeln!(
                        writer,
                        "M106 S{} ; set fan speed",
                        (2.550 * fan_speed).round() as usize
                    )?;
                }
            }
            Command::LayerChange { z, index } => {
                navigator.record_layer_change(writer.line_count());

                writeln!(writer, ";LAYER:{}", *index)?;

                writeln!(
                    writer,
                    "{}",
                    convert_instructions(
                        settings.before_layer_change_instructions.clone(),
                        current_z,
                        layer_count,
                        None,
                        current_object,
                        settings
                    )
                )?;
                current_z = *z;
                layer_count = *index;
                writeln!(writer, "G1 Z{:.5}", z)?;

                writeln!(
                    writer,
                    "{}",
                    convert_instructions(
                        settings.after_layer_change_instructions.clone(),
                        current_z,
                        layer_count,
                        None,
                        current_object,
                        settings
                    )
                )?;
            }
            Command::Delay { msec } => {
                writeln!(writer, "G4 P{:.5}", msec)?;
            }
            Command::Arc {
                start,
                end,
                center,
                clockwise,
                width,
                thickness,
            } => {
                let x_diff = end.x - start.x;
                let y_diff = end.y - start.y;
                let cord_length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                let x_diff_r = end.x - center.x;
                let y_diff_r = end.y - center.y;
                let radius = ((x_diff_r * x_diff_r) + (y_diff_r * y_diff_r)).sqrt();

                //Divide the chord length by double the radius.
                let t = cord_length / (2.0 * radius);
                //println!("{}",t);
                //Find the inverse sine of the result (in radians).
                //Double the result of the inverse sine to get the central angle in radians.
                let central = t.asin() * 2.0;
                //Once you have the central angle in radians, multiply it by the radius to get the arc length.
                let extrusion_length = central * radius;

                //println!("{}",extrusion_length);
                let extrude = (4.0 * thickness * width * extrusion_length)
                    / (std::f32::consts::PI
                        * settings.filament.diameter
                        * settings.filament.diameter);
                writeln!(
                    writer,
                    "{} X{:.5} Y{:.5} I{:.5} J{:.5} E{:.5}",
                    if *clockwise { "G2" } else { "G3" },
                    end.x,
                    end.y,
                    center.x - start.x,
                    center.y - start.y,
                    extrude
                )?;
            }
            Command::ChangeObject { object } => {
                let previous_object = std::mem::replace(&mut current_object, Some(*object));
                writeln!(
                    writer,
                    "{}",
                    convert_instructions(
                        settings.object_change_instructions.clone(),
                        current_z,
                        layer_count,
                        previous_object,
                        current_object,
                        settings
                    )
                )?;
            }
            Command::NoAction => {
                panic!("Converter reached a No Action Command, Optimization Failure")
            }
            Command::ChangeType { print_type } => {
                writeln!(writer, ";TYPE:{}", print_type)?;
            }
        }
    }

    let end = convert_instructions(
        settings.ending_instructions.clone(),
        current_z,
        layer_count,
        None,
        current_object,
        settings,
    );

    writeln!(writer, "{}", end)?;

    writer
        .flush()
        .expect("File Closed Before Closed. Gcode invalid.");

    Ok(navigator)
}

fn convert_instructions(
    mut instructions: String,
    current_z_height: f32,
    layer_count: usize,
    previous_object: Option<usize>,
    current_object: Option<usize>,
    settings: &Settings,
) -> String {
    let layer_settings = settings.get_layer_settings(layer_count, current_z_height);

    instructions = instructions.replace(
        "[Extruder Temperature]",
        &format!("{:.1}", layer_settings.extruder_temp),
    );

    instructions = instructions.replace(
        "[Bed Temperature]",
        &format!("{:.1}", layer_settings.bed_temp),
    );

    instructions = instructions.replace("[Z Position]", &format!("{:.5}", current_z_height));

    instructions = instructions.replace("[Layer Count]", &format!("{:.1}", layer_count));

    instructions = instructions.replace(
        "[Previous Object]",
        &previous_object
            .map(|obj| obj.to_string())
            .unwrap_or_default(),
    );

    instructions = instructions.replace(
        "[Current Object]",
        &current_object
            .map(|obj| obj.to_string())
            .unwrap_or_default(),
    );

    instructions
}

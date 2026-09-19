use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Graphics {
    pub device: GraphicsDevice,
    pub display: Display,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, strum::Display, PartialEq, Deserialize, Serialize)]
pub enum Display {
    #[default]
    None,
    Gtk,
    Sdl,
    EglHeadless,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, strum::Display, PartialEq, Deserialize, Serialize)]
pub enum GraphicsDevice {
    Std { vgamem: u16 },
    Qxl { vgamem: u16 },
    Virtio(VirtioConfig),
    None,
}

impl Default for GraphicsDevice {
    fn default() -> Self {
        GraphicsDevice::Std { vgamem: 16 }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
pub struct VirtioConfig {
    pub enable_3d: bool,
    pub venus: bool,
}

impl Graphics {
    pub fn to_qemu_arg(&self) -> Vec<String> {
        let display = match self.display {
            Display::None => vec!["-display".to_string(), "none".to_string()],
            Display::EglHeadless => vec!["-display".to_string(), "egl-headless".to_string()],
            Display::Sdl => vec!["-display".to_string(), "sdl".to_string()],
            Display::Gtk => vec!["-display".to_string(), "gtk".to_string()],
        };

        let device = match self.device {
            GraphicsDevice::Std { vgamem } => {
                vec![
                    "-vga".to_string(),
                    "none".to_string(),
                    "-device".to_string(),
                    format!("VGA,vgamem_mb={}", vgamem),
                ]
            }
            GraphicsDevice::Qxl { vgamem } => {
                vec![
                    "-vga".to_string(),
                    "none".to_string(),
                    "-device".to_string(),
                    format!("qxl-vga,vgamem_mb={}", vgamem),
                ]
            }
            GraphicsDevice::Virtio(config) => {
                let mut arg = vec!["-vga".to_string(), "none".to_string()];

                if config.enable_3d {
                    if config.venus {
                        arg.extend(vec![
                            "-device".to_string(),
                            "virtio-vga-gl,hostmem=8G,blob=true,venus=true".to_string(),
                        ]);
                    } else {
                        arg.extend(vec![
                            "-device".to_string(),
                            "virtio-vga-gl,hostmem=8G,blob=true".to_string(),
                        ]);
                    }
                } else {
                    arg.extend(vec!["-device".to_string(), "virtio-vga".to_string()]);
                }

                arg
            }
            GraphicsDevice::None => {
                vec!["-vga".to_string(), "none".to_string()]
            }
        };

        let mut args = Vec::new();
        args.extend(display);
        args.extend(device);
        args
    }
}

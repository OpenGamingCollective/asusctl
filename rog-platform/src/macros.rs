#[macro_export]
macro_rules! has_attr {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = has_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self) -> bool {
                match to_device(&self.$item) {
                    Ok(p) => $crate::has_attr(&p, $attr_name),
                    Err(_) => false,
                }
            }
        });
    };
}

#[macro_export]
macro_rules! watch_attr {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = monitor_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self) -> Result<inotify::Inotify> {
                let mut path = self.$item.clone();
                path.push($attr_name);
                if let Some(path) = path.to_str() {
                    let inotify = inotify::Inotify::init()?;
                    inotify.watches().add(path, inotify::WatchMask::MODIFY)
                        .map_err(|e| {
                            if e.kind() == std::io::ErrorKind::NotFound {
                                PlatformError::AttrNotFound(format!("{}", $attr_name))
                            } else {
                                PlatformError::IoPath(format!("{}", path), e)
                            }
                        })?;
                    return Ok(inotify);
                }
                Err(PlatformError::AttrNotFound(format!("{}", $attr_name)))
            }
        });
    };
}

#[macro_export]
macro_rules! get_attr_bool {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = get_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self) -> Result<bool> {
                $crate::read_attr_bool(&to_device(&self.$item)?, $attr_name)
            }
        });
    };
}

#[macro_export]
macro_rules! set_attr_bool {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = set_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self, value: bool) -> Result<()> {
                $crate::write_attr_bool(&mut to_device(&self.$item)?, $attr_name, value)
            }
        });
    };
}

#[macro_export]
macro_rules! attr_bool {
    ($(#[$attr:meta])* $attr_name:literal, $item:ident) => {
        $crate::has_attr!($attr_name $item);
        $crate::get_attr_bool!( $attr_name $item);
        $crate::set_attr_bool!($attr_name $item);
        $crate::watch_attr!($attr_name $item);
    };
}

#[macro_export]
macro_rules! get_attr_num {
    ($(#[$attr:meta])* $attr_name:literal $item:ident $type:ty) => {
        concat_idents::concat_idents!(fn_name = get_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self) -> Result<$type> {
                $crate::read_attr_num::<$type>(&to_device(&self.$item)?, $attr_name)
            }
        });
    };
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        $crate::get_attr_num!($(#[$attr])* $attr_name $item $type);
    };
}

#[macro_export]
macro_rules! set_attr_num {
    ($(#[$attr:meta])* $attr_name:literal $item:ident $type:ty) => {
        concat_idents::concat_idents!(fn_name = set_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self, value: $type) -> Result<()> {
                $crate::write_attr_num(&mut to_device(&self.$item)?, $attr_name, value as $type)
            }
        });
    };
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        $crate::set_attr_num!($(#[$attr])* $attr_name $item $type);
    };
}

#[macro_export]
macro_rules! attr_num {
    ($(#[$attr:meta])* $attr_name:literal, $item:ident, $type:ty) => {
        $crate::has_attr!($(#[$attr])* $attr_name $item);
        $crate::get_attr_num!($(#[$attr])* $attr_name $item $type);
        $crate::set_attr_num!($(#[$attr])* $attr_name $item $type);
        $crate::watch_attr!($(#[$attr])* $attr_name $item);
    };
}

#[macro_export]
macro_rules! get_attr_u8_array {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = get_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self) -> Result<Vec<u8>> {
                $crate::read_attr_u8_array(&to_device(&self.$item)?, $attr_name)
            }
        });
    };
}

#[macro_export]
macro_rules! set_attr_u8_array {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = set_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self, values: &[u8]) -> Result<()> {
                $crate::write_attr_u8_array(&mut to_device(&self.$item)?, $attr_name, values)
            }
        });
    };
}

#[macro_export]
macro_rules! attr_u8_array {
    ($(#[$attr:meta])* $attr_name:literal, $item:ident) => {
        $crate::has_attr!($attr_name $item);
        $crate::get_attr_u8_array!($attr_name $item);
        $crate::set_attr_u8_array!($attr_name $item);
        $crate::watch_attr!($attr_name $item);
    };
}

#[macro_export]
macro_rules! get_attr_string {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = get_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self) -> Result<String> {
                match to_device(&self.$item) {
                    Ok(dev) => $crate::read_attr_string(&dev, $attr_name),
                    Err(_) => {
                        let path = self.$item.join($attr_name);
                        std::fs::read_to_string(&path)
                            .map_err(|e| PlatformError::IoPath(path.to_string_lossy().into(), e))
                    }
                }
            }
        });
    };
}

#[macro_export]
macro_rules! get_attr_string_array {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = get_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self) -> Result<Vec<PlatformProfile>> {
                match to_device(&self.$item) {
                    Ok(dev) => $crate::read_attr_string_array(&dev, $attr_name),
                    Err(_) => {
                        let path = self.$item.join($attr_name);
                        let s = std::fs::read_to_string(&path)
                            .map_err(|e| PlatformError::IoPath(path.to_string_lossy().into(), e))?;
                        let tmp: Vec<PlatformProfile> = s
                            .split(' ')
                            .map(PlatformProfile::from)
                            .collect();
                        Ok(tmp)
                    }
                }
            }
        });
    };
}

#[macro_export]
macro_rules! set_attr_string {
    ($(#[$attr:meta])* $attr_name:literal $item:ident) => {
        concat_idents::concat_idents!(fn_name = set_, $attr_name {
            $(#[$attr])*
            pub fn fn_name(&self, values: &str) -> Result<()> {
                match to_device(&self.$item) {
                    Ok(mut dev) => $crate::write_attr_string(&mut dev, $attr_name, values),
                    Err(_) => {
                        let path = self.$item.join($attr_name);
                        std::fs::write(&path, values.trim())
                            .map_err(|e| PlatformError::IoPath(path.to_string_lossy().into(), e))
                    }
                }
            }
        });
    };
}

#[macro_export]
macro_rules! attr_string {
    ($(#[$attr:meta])* $attr_name:literal, $item:ident) => {
        $crate::has_attr!($attr_name $item);
        $crate::get_attr_string!($attr_name $item);
        $crate::set_attr_string!($attr_name $item);
        $crate::watch_attr!($attr_name $item);
    };
}

#[macro_export]
macro_rules! attr_string_array {
    ($(#[$attr:meta])* $attr_name:literal, $item:ident) => {
        $crate::has_attr!($attr_name $item);
        $crate::get_attr_string_array!($attr_name $item);
        $crate::watch_attr!($attr_name $item);
    };
}

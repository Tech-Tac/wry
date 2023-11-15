use std::cell::Cell;

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use servo::{
  compositing::windowing::{AnimationState, EmbedderCoordinates, WindowMethods},
  config::pref,
  euclid::{Point2D, Scale, Size2D},
  webrender_api::units::DeviceIntRect,
  webrender_surfman::WebrenderSurfman,
};
use surfman::{Connection, GLApi, GLVersion, SurfaceType};
// FIXME servo should re-export this.
use servo_media::player::context::{GlApi, GlContext, NativeDisplay};

/// This is the type for servo embedder. Not for public usage.
pub struct Window {
  webrender_surfman: WebrenderSurfman,
  animation_state: Cell<AnimationState>,
}

impl Window {
  pub fn new(window_handle: RawWindowHandle) -> Self {
    let connection = Connection::new().expect("Failed to create surfman connection");
    let adapter = connection
      .create_adapter()
      .expect("Failed to create surfman adapter");
    let native_widget = connection
      .create_native_widget_from_rwh(window_handle)
      .expect("Failed to create surfman native widget");
    let surface_type = SurfaceType::Widget { native_widget };
    let webrender_surfman = WebrenderSurfman::create(&connection, &adapter, surface_type)
      .expect("Failed to create webrender surfman");
    log::trace!("Created webrender surfman for window {:?}", window_handle);

    Self {
      webrender_surfman,
      animation_state: Cell::new(AnimationState::Idle),
    }
  }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl WindowMethods for Window {
  fn get_coordinates(&self) -> EmbedderCoordinates {
    //TODO
    EmbedderCoordinates {
      hidpi_factor: Scale::new(1.0),
      screen: Size2D::new(1980, 720),
      screen_avail: Size2D::new(1980, 720),
      window: (Size2D::new(400, 400), Point2D::new(0, 0)),
      framebuffer: Size2D::new(400, 400),
      viewport: DeviceIntRect::new(Point2D::new(0, 0), Size2D::new(400, 400)),
    }
  }

  fn set_animation_state(&self, state: AnimationState) {
    self.animation_state.set(state);
  }

  fn get_gl_context(&self) -> GlContext {
    if !pref!(media.glvideo.enabled) {
      return GlContext::Unknown;
    }

    #[allow(unused_variables)]
    let native_context = self.webrender_surfman.native_context();

    #[cfg(target_os = "windows")]
    return PlayerGLContext::Egl(native_context.egl_context as usize);

    #[cfg(target_os = "linux")]
    return {
      use surfman::platform::generic::multi::context::NativeContext;
      match native_context {
        NativeContext::Default(NativeContext::Default(native_context)) => {
          GlContext::Egl(native_context.egl_context as usize)
        }
        NativeContext::Default(NativeContext::Alternate(native_context)) => {
          GlContext::Egl(native_context.egl_context as usize)
        }
        NativeContext::Alternate(_) => unimplemented!(),
      }
    };

    // @TODO(victor): https://github.com/servo/media/pull/315
    #[cfg(target_os = "macos")]
    #[allow(unreachable_code)]
    return unimplemented!();

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return unimplemented!();
  }

  fn get_native_display(&self) -> NativeDisplay {
    if !pref!(media.glvideo.enabled) {
      return NativeDisplay::Unknown;
    }

    #[allow(unused_variables)]
    let native_connection = self.webrender_surfman.connection().native_connection();
    #[allow(unused_variables)]
    let native_device = self.webrender_surfman.native_device();

    #[cfg(target_os = "windows")]
    return NativeDisplay::Egl(native_device.egl_display as usize);

    #[cfg(target_os = "linux")]
    return {
      use surfman::platform::generic::multi::connection::NativeConnection;
      match native_connection {
        NativeConnection::Default(NativeConnection::Default(conn)) => {
          NativeDisplay::Egl(conn.0 as usize)
        }
        NativeConnection::Default(NativeConnection::Alternate(conn)) => {
          NativeDisplay::X11(conn.x11_display as usize)
        }
        NativeConnection::Alternate(_) => unimplemented!(),
      }
    };

    // @TODO(victor): https://github.com/servo/media/pull/315
    #[cfg(target_os = "macos")]
    #[allow(unreachable_code)]
    return unimplemented!();

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return unimplemented!();
  }

  fn get_gl_api(&self) -> GlApi {
    let api = self.webrender_surfman.connection().gl_api();
    let attributes = self.webrender_surfman.context_attributes();
    let GLVersion { major, minor } = attributes.version;
    match api {
      GLApi::GL if major >= 3 && minor >= 2 => GlApi::OpenGL3,
      GLApi::GL => GlApi::OpenGL,
      GLApi::GLES if major > 1 => GlApi::Gles2,
      GLApi::GLES => GlApi::Gles1,
    }
  }

  fn webrender_surfman(&self) -> WebrenderSurfman {
    self.webrender_surfman.clone()
  }
}

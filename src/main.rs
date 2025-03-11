use gstreamer::Pipeline;
use gstreamer::prelude::{ElementExt as _, GstBinExt};
use gtk4::{Application, ApplicationWindow, Box as GtkBox};
use gtk4::{gdk, prelude::*};
use std::rc::Rc;
use gstreamer::State;

mod picture;

struct Config {
    camera: CameraConfig,
}

struct CameraConfig {
    width: i32,
    height: i32,
    fps: i32,
}

fn main() {
    // Создаем новое приложение с уникальным идентификатором
    let app = Application::new(Some("com.example.MyGTKApp"), Default::default());

    let config = Config {
        camera: CameraConfig {
            width: 480,
            height: 320,
            fps: 25,
        },
    };



    // Инициализируем GStreamer
    gstreamer::init().unwrap();

        gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

    // Создаем pipeline для вывода видео с параметрами YUY2 (480x320, 25 fps)
    let pipeline_str = format!(
        "v4l2src device=/dev/video0 ! video/x-raw,format=YUY2,width={},height={},framerate={}/1 ! videoconvert ! video/x-raw,format=BGRA ! gtk4paintablesink name=sink1",
        config.camera.width, config.camera.height, config.camera.fps,
    );
    // Парсим строку pipeline и создаем объект pipeline
    let pipeline =
        gstreamer::parse::launch(&pipeline_str).expect("Can not create GStreamer with pipeline");
    let pipeline = pipeline
        .dynamic_cast::<Pipeline>()
        .expect("Can not dynamic_cast pipeline");

    // Получаем элемент gtk4paintablesink из pipeline
    let gtksink = pipeline
        .by_name("sink1")
        .expect("Can not get gtk4paintablesink element");

    // Устанавливаем обработчик события активации приложения
    app.connect_activate(move |app| {
        // Создаем новое окно приложения
        let window = ApplicationWindow::new(app);
        window.set_title(Some("My GTK App")); // Устанавливаем заголовок окна
        window.set_default_size(1024, 600); // Устанавливаем размер окна по умолчанию

        // Создаем вертикальный бокс для размещения виджетов
        let vbox = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox.set_halign(gtk4::Align::Center); // Выравниваем бокс по горизонтали по центру
        vbox.set_valign(gtk4::Align::Center); // Выравниваем бокс по вертикали по центру

        // Создаем экземпляр структуры Picture и добавляем его в бокс
        let paintable = gtksink.property::<gdk::Paintable>("paintable");
        let picture = gtk4::Picture::new();

        picture.set_paintable(Some(&paintable));

        vbox.append(&picture);

        // Устанавливаем бокс как дочерний элемент окна
        window.set_child(Some(&vbox));
        window.show(); // Показываем окно
        pipeline
            .set_state(State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
    });

    // Запускаем приложение
    app.run();
}

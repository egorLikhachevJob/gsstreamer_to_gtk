use gstreamer::Pipeline;
use gstreamer::State;
use gstreamer::prelude::{ElementExt as _, GstBinExt};
use gtk4::gdk::Display;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, CssProvider};
use gtk4::{gdk, prelude::*};

mod picture;

struct Config {
    camera: CameraConfig,
}

struct CameraConfig {
    width: i32,
    height: i32,
    fps: i32,
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_file(&gtk4::gio::File::for_path("src/style.css"));

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
fn main() {
    // Создаем новое приложение с уникальным идентификатором
    let app = Application::new(Some("com.example.MyGTKApp"), Default::default());
    app.connect_startup(|_| load_css());

    let config = Config {
        camera: CameraConfig {
            width: 720,
            height: 480,
            fps: 25,
        },
    };

    // Инициализируем GStreamer
    gstreamer::init().unwrap();

    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

    // Создаем pipeline для вывода видео с параметрами jpeg (720x480, 25 fps)
    let pipeline_str = format!(
        "v4l2src device=/dev/video0 ! image/jpeg,width={},height={},framerate={}/1 ! jpegdec ! videoconvert ! video/x-raw,format=BGRA ! gtk4paintablesink name=sink1",
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
        // Создаем новый окно приложения
        let window = ApplicationWindow::new(app);
        window.set_title(Some("My GTK App")); // Устанавливаем заголовок окна
        window.set_default_size(1024, 600); // Устанавливаем размер окна по умолчанию

        // Создаем вертикальный бокс для размещения виджетов
        let hbox = GtkBox::new(gtk4::Orientation::Horizontal, 5);
        hbox.set_halign(gtk4::Align::Center); // Выравниваем бокс по горизонтали по центру
        hbox.set_valign(gtk4::Align::Center); // Выравниваем бокс по вертикали по центру
        hbox.add_css_class("screen_box");

        let vbox1 = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox1.set_hexpand(true);
        vbox1.set_vexpand(true);

        let vbox2 = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox2.set_hexpand(true);
        vbox2.set_vexpand(true);
        vbox2.set_size_request(720, 480);

        let vbox3 = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox3.set_hexpand(true);
        vbox3.set_vexpand(true);

        // Создаем кнопки
        let button1 = Button::with_label("Сканер Частоты");
        let button2 = Button::with_label("Ввод Позывного");
        let button3 = Button::with_label("  Бинд Фраза  ");
        let button_rec = Button::with_label(" Запись видео ");

        // Устанавливаем кнопки для расширения и заполнения доступного пространства
        button1.set_hexpand(true);
        button1.set_vexpand(true);
        button2.set_hexpand(true);
        button2.set_vexpand(true);
        button3.set_hexpand(true);
        button3.set_vexpand(true);
        button_rec.set_hexpand(true);
        button_rec.set_vexpand(true);

        // Создаем экземпляр структуры Picture
        let paintable = gtksink.property::<gdk::Paintable>("paintable");
        let picture = gtk4::Picture::new();
        picture.set_paintable(Some(&paintable));

        // Добавляем кнопки и картинку в горизонтальный бокс
        vbox1.append(&button1);
        vbox1.append(&button2);
        vbox2.append(&picture);
        vbox3.append(&button3);
        vbox3.append(&button_rec);

        // Добавляем горизонтальный бокс в вертикальный бокс
        hbox.append(&vbox1);
        hbox.append(&vbox2);
        hbox.append(&vbox3);

        // Устанавливаем бокс как дочерний элемент окна
        window.set_child(Some(&hbox));
        window.show(); // Показываем окно

        // Загружаем CSS стили из файла

        pipeline
            .set_state(State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
    });

    // Запускаем приложение
    app.run();
}

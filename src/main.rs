use gstreamer::Pipeline;
use gstreamer::State;
use gstreamer::prelude::{ElementExt as _, GstBinExt};
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button};
use gtk4::{gdk, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

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
        // Создаем новое окно приложения
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
        let button4 = Button::with_label(" Запись видео ");

        // Устанавливаем кнопки для расширения и заполнения доступного пространства
        button1.set_hexpand(true);
        button1.set_vexpand(true);
        button2.set_hexpand(true);
        button2.set_vexpand(true);
        button3.set_hexpand(true);
        button3.set_vexpand(true);
        button4.set_hexpand(true);
        button4.set_vexpand(true);

        // Создаем экземпляр структуры Picture
        let paintable = gtksink.property::<gdk::Paintable>("paintable");
        let picture = gtk4::Picture::new();
        picture.set_paintable(Some(&paintable));

        // Добавляем кнопки и картинку в горизонтальный бокс
        vbox1.append(&button1);
        vbox1.append(&button2);
        vbox2.append(&picture);
        vbox3.append(&button3);
        vbox3.append(&button4);

        // Добавляем горизонтальный бокс в вертикальный бокс
        hbox.append(&vbox1);
        hbox.append(&vbox2);
        hbox.append(&vbox3);

        // Устанавливаем бокс как дочерний элемент окна
        window.set_child(Some(&hbox));
        window.show(); // Показываем окно
        pipeline
            .set_state(State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");

        // Создаем RefCell для хранения дополнительного окна
        let additional_window: Rc<RefCell<Option<ApplicationWindow>>> = Rc::new(RefCell::new(None));

        // Обработчик события нажатия на кнопку button1
        let additional_window_clone1 = additional_window.clone();
        let app_clone = app.clone();
        button1.connect_clicked(move |_| {
            let mut additional_window = additional_window_clone1.borrow_mut();
            if additional_window.is_none() {
                let new_window = ApplicationWindow::new(&app_clone);
                new_window.set_title(Some("Сканер Частоты"));
                new_window.set_default_size(1024, 600);
                new_window.show();

                // Закрываем дополнительное окно при его закрытии
                let additional_window_clone = additional_window_clone1.clone();
                new_window.connect_close_request(move |_| {
                    *additional_window_clone.borrow_mut() = None;
                    false.into()
                });
                *additional_window = Some(new_window);
            }
        });

        // Обработчик события нажатия на кнопку button2
        let additional_window_clone2 = additional_window.clone();
        let app_clone = app.clone();
        button2.connect_clicked(move |_| {
            let mut additional_window = additional_window_clone2.borrow_mut();
            if additional_window.is_none() {
                let new_window = ApplicationWindow::new(&app_clone);
                new_window.set_title(Some("Ввод Позывного"));
                new_window.set_default_size(1024, 600);
                new_window.show();

                // Закрываем дополнительное окно при его закрытии
                let additional_window_clone = additional_window_clone2.clone();
                new_window.connect_close_request(move |_| {
                    *additional_window_clone.borrow_mut() = None;
                    false.into()
                });
                *additional_window = Some(new_window);
            }
        });

        // Обработчик события нажатия на кнопку button3
        let additional_window_clone3 = additional_window.clone();
        let app_clone = app.clone();
        button3.connect_clicked(move |_| {
            let mut additional_window = additional_window_clone3.borrow_mut();
            if additional_window.is_none() {
                let new_window = ApplicationWindow::new(&app_clone);
                new_window.set_title(Some("Бинд Фраза"));
                new_window.set_default_size(1024, 600);
                new_window.show();

                // Закрываем дополнительное окно при его закрытии
                let additional_window_clone = additional_window_clone3.clone();
                new_window.connect_close_request(move |_| {
                    *additional_window_clone.borrow_mut() = None;
                    false.into()
                });
                *additional_window = Some(new_window);
            }
        });
    });

    // Запускаем приложение
    app.run();
}
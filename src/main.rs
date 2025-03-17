use crate::gdk::Texture;
use chrono::prelude::*;
use glib::timeout_add_local;
use gstreamer::Pipeline;
use gstreamer::State;
use gstreamer::prelude::{ElementExt as _, GstBinExt};
use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, Picture};
use gtk4::{gdk, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::fs;
use std::path::Path; 

mod picture;

struct Config {
    camera: CameraConfig,
}

struct CameraConfig {
    width: i32,
    height: i32,
    fps: i32,
    path: String,
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
            path: String::from("src/media/"),
        },
    };

    // Инициализируем GStreamer
    gstreamer::init().unwrap();

    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

    // Проверяем и создаём директорию, если она не существует
    let media_path = Path::new(&config.camera.path);
    if !media_path.exists() {
        fs::create_dir_all(media_path).expect("Failed to create media directory");
    }

    let now = Utc::now();
    let formatted_date_time = now.format("%Y-%m-%d|%H:%M:%S").to_string();

    // Создаем pipeline для вывода видео с параметрами jpeg (720x480, 25 fps)
    let pipeline_str = format!(
        "v4l2src device=/dev/video0 ! image/jpeg,width={},height={},framerate={}/1 ! tee name=t t. ! jpegdec ! videoconvert ! video/x-raw,format=BGRA 
        ! queue ! gtk4paintablesink name=sink1 t. ! queue ! avimux 
        ! filesink location={}{}.mp4",
        config.camera.width, config.camera.height, config.camera.fps, config.camera.path, formatted_date_time,
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
        window.set_default_size(1024, 600);
        window.set_decorated(false); // Устанавливаем размер окна по умолчанию

        // Создаем вертикальный бокс для размещения виджетов
        let hbox = GtkBox::new(gtk4::Orientation::Horizontal, 5);
        hbox.set_halign(gtk4::Align::Center); // Выравниваем бокс по горизонтали по центру
        hbox.set_valign(gtk4::Align::Center); // Выравниваем бокс по вертикали по центру
        hbox.add_css_class("screen_box");

        let vbox1 = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox1.set_hexpand(true);
        vbox1.set_vexpand(true);

        let display_window = GtkBox::new(gtk4::Orientation::Vertical, 5);
        display_window.set_hexpand(true);
        display_window.set_vexpand(true);
        display_window.set_size_request(720, 480);

        let vbox3 = GtkBox::new(gtk4::Orientation::Vertical, 5);
        vbox3.set_hexpand(true);
        vbox3.set_vexpand(true);

        // Создаем кнопки
        let button1 = Button::with_label("Сканер Частоты");
        let button2 = Button::with_label("Ввод Позывного");
        let button3 = Button::with_label("  Бинд Фраза  ");
        let button_rec = Button::with_label(" Запись видео ");
        button_rec.add_css_class("rec_btn");

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
        let picture = Picture::new();
        picture.set_paintable(Some(&paintable));

        // Добавляем кнопки и картинку в вертикальный бокс
        vbox1.append(&button1);
        vbox1.append(&button2);
        display_window.append(&picture);
        vbox3.append(&button3);
        vbox3.append(&button_rec);

        // Добавляем вертикальный бокс в горизонтальный бокс
        hbox.append(&vbox1);
        hbox.append(&display_window);
        hbox.append(&vbox3);

        // Устанавливаем бокс как дочерний элемент окна
        window.set_child(Some(&hbox));
        window.show(); // Показываем окно

        pipeline
            .set_state(State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");

        let picture = Rc::new(RefCell::new(picture));
        let paintable = Rc::new(RefCell::new(paintable));

        // Обработчик нажатия кнопки button1
        button1.connect_clicked({
            let display_window = display_window.clone();
            let picture = picture.clone();
            move |_| {
                println!("Button 1 clicked");
                let file = gtk4::gio::File::for_path("src/images/dog.jpg");
                let texture = Texture::from_file(&file).expect("Failed to load image");
                let new_picture = Picture::new();
                new_picture.set_paintable(Some(&texture));
                let _ = display_window.remove(&*picture.borrow());
                *picture.borrow_mut() = new_picture;
                display_window.append(&*picture.borrow());
            }
        });

        button2.connect_clicked({
            let display_window = display_window.clone();
            let picture = picture.clone();
            move |_| {
                println!("Button 2 clicked");
                let file = gtk4::gio::File::for_path("src/images/cat.jpg");
                let texture = Texture::from_file(&file).expect("Failed to load image");
                let new_picture = Picture::new();
                new_picture.set_paintable(Some(&texture));
                let _ = display_window.remove(&*picture.borrow());
                *picture.borrow_mut() = new_picture;
                display_window.append(&*picture.borrow());
            }
        });

        button3.connect_clicked({
            let display_window = display_window.clone();
            let picture = picture.clone();
            move |_| {
                println!("Button 3 clicked");
                let file = gtk4::gio::File::for_path("src/images/tiger.jpg");
                let texture = Texture::from_file(&file).expect("Failed to load image");
                let new_picture = Picture::new();
                new_picture.set_paintable(Some(&texture));
                let _ = display_window.remove(&*picture.borrow());
                *picture.borrow_mut() = new_picture;
                display_window.append(&*picture.borrow());
            }
        });

        // Обработчик нажатия кнопки button_rec
        button_rec.connect_clicked({
            let display_window = display_window.clone();
            let picture = picture.clone();
            let paintable = paintable.clone();
            move |_| {
                println!("Button rec clicked");
                let file = gtk4::gio::File::for_path("src/images/racoon.jpg");
                let texture = Texture::from_file(&file).expect("Failed to load image");
                let new_picture = Picture::new();
                new_picture.set_paintable(Some(&texture));
                let _ = display_window.remove(&*picture.borrow());
                *picture.borrow_mut() = new_picture;
                display_window.append(&*picture.borrow());

                // Запускаем таймер для восстановления исходной картинки через 1 секунду
                let display_window = display_window.clone();
                let picture = picture.clone();
                let paintable = paintable.clone();
                timeout_add_local(Duration::from_secs(1), move || {
                    let _ = display_window.remove(&*picture.borrow());
                    let new_picture = Picture::new();
                    new_picture.set_paintable(Some(&*paintable.borrow()));
                    *picture.borrow_mut() = new_picture;
                    display_window.append(&*picture.borrow());
                    glib::ControlFlow::Break // Исправлено здесь
                });
            }
        });
    });

    // Запускаем приложение
    app.run();
}

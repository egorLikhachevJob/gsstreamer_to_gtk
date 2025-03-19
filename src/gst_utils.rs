use gstreamer::prelude::*;
use gstreamer::{
    Bin, Bus, Element, MessageView, Pipeline, State,
};
use std::error::Error;

/// Отключает ветку от tee
pub fn unlink_tee_branch(
    pipeline: &Pipeline,
    tee: &Element,
    branch: &Bin,
    callback: Box<dyn Fn()>,
) {
    // Переводим branch в состояние Null
    let _ = branch.set_state(State::Null);

    // Получаем все src pads от tee
    if let Some(tee_src_pad) = tee.pads().into_iter().find(|p| {
        p.direction() == gstreamer::PadDirection::Src
            && p.is_linked()
            && p.peer().map_or(false, |peer| {
                if let Some(parent) = peer.parent() {
                    parent.downcast_ref::<Bin>().map_or(false, |b| b == branch)
                } else {
                    false
                }
            })
    }) {
        // Отключаем pad
        if let Some(peer) = tee_src_pad.peer() {
            let _ = tee_src_pad.unlink(&peer);
        }
        // Освобождаем pad
        tee.release_request_pad(&tee_src_pad);
    }

    // Удаляем branch из pipeline
    let _ = pipeline.remove(branch);

    // Вызываем callback
    callback();
}

/// Подключает ветку к tee
pub fn link_tee_branch(
    pipeline: &Pipeline,
    tee: &Element,
    branch: &Bin,
) -> Result<(), Box<dyn Error>> {
    println!("Начинаем подключение ветки к tee");

    // Получаем свободный src pad от tee
    let tee_src_pad = tee
        .request_pad_simple("src_%u")
        .ok_or("Can not get teepad")?;
    println!("Получен src pad от tee: {:?}", tee_src_pad.name());

    // Добавляем branch в pipeline
    pipeline.add(branch)?;
    println!("Branch добавлен в pipeline");

    // Получаем sink pad от branch
    let sink_pad = branch
        .static_pad("sink")
        .ok_or("Не удалось получить sink pad")?;
    println!("Получен sink pad от branch: {:?}", sink_pad.name());

    // Линкуем tee с branch
    tee_src_pad.link(&sink_pad)?;
    println!("Pads успешно соединены");

    // Устанавливаем состояние branch в Playing
    branch.sync_state_with_parent()?;
    println!("Branch синхронизирован с pipeline");

    Ok(())
}

#[allow(dead_code)]
pub fn dispatch_messages(bus: &Bus, pipeline: &Pipeline) -> bool {
    while let Some(msg) = bus.pop() {
        match msg.view() {
            MessageView::Eos(..) => return false,
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                pipeline
                    .set_state(State::Null)
                    .expect("Unable to set the pipeline to the Null state");
                return false;
            }
            MessageView::StateChanged(state_changed) => {
                println!(
                    "State changed from {:?}: {:?} -> {:?}",
                    state_changed.src().map(|s| s.path_string()),
                    state_changed.old(),
                    state_changed.current()
                );
            }
            _ => {}
        }
    }
    true
}


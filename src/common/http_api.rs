use eframe::egui;
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, RwLock},
    thread,
};

pub struct CachedHttpApi<Key, Value, Response> {
    cache: Arc<RwLock<HashMap<Key, Option<Arc<Value>>>>>,
    context: egui::Context,
    url_maker: Box<dyn Fn(&Key) -> String>,
    response_to_value: Arc<dyn Fn(Response) -> Option<Value> + Send + Sync>,
    agent: ureq::Agent,
}

impl<Key, Value, Response> CachedHttpApi<Key, Value, Response>
where
    Key: Eq + Hash + Clone + Send + Sync + 'static,
    Value: Send + Sync + 'static,
    Response: DeserializeOwned + 'static,
{
    pub fn new(
        context: egui::Context,
        url_maker: Box<dyn Fn(&Key) -> String>,
        response_to_value: Arc<dyn Fn(Response) -> Option<Value> + Send + Sync>,
    ) -> Self {
        let ureq_config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build();

        Self {
            cache: Arc::default(),
            context,
            url_maker,
            response_to_value,
            agent: ureq_config.into(),
        }
    }

    pub fn get(&self, key: &Key) -> Option<Arc<Value>> {
        let current = Arc::clone(&self.cache);
        if let Some(existing) = current.read().unwrap().get(key) {
            return existing.clone();
        }
        current.write().unwrap().insert(key.clone(), None);

        let player_id = key.clone();
        let context = self.context.clone();

        let url = (self.url_maker)(&player_id);
        let response_to_value = Arc::clone(&self.response_to_value);
        let agent = self.agent.clone();

        thread::spawn(move || {
            let Ok(mut response) = agent.get(&url).call() else {
                let mut current = current.write().unwrap();
                current.remove(&player_id);
                return;
            };

            if response.status() != 200 {
                let mut current = current.write().unwrap();
                current.insert(player_id, None);
                eprintln!("http error {}", response.status().as_u16());
                return;
            }

            let response: Response = response.body_mut().read_json().unwrap();
            let mut current = current.write().unwrap();
            current.insert(player_id, response_to_value(response).map(Arc::new));
            context.request_repaint();
        });

        None
    }

    pub fn invalidate<'a>(&self, keys: impl IntoIterator<Item = &'a Key>) {
        let mut current = self.cache.write().unwrap();
        for key in keys {
            current.remove(key);
        }
    }
}

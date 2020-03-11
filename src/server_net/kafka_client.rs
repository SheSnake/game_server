use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::message::{Headers, Message};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::util::get_rdkafka_version;
use tokio::stream::StreamExt;
use redis::AsyncCommands;
use rdkafka::message::OwnedHeaders;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tokio::sync::mpsc::{ channel, Sender };
use futures::*;
use std::collections::HashMap;
use tokio::sync::{ Mutex };
use std::sync::Arc;

pub struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        println!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        println!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        //println!("Committing offsets: {:?}", result);
    }
}
use chrono::prelude::*;

pub type LoggingConsumer = StreamConsumer<CustomContext>;

pub async fn consume_and_print(brokers: &str, group_id: &str, topics: &[&str], mut sender: Sender<Vec<u8>>) {
    let context = CustomContext;

    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");

    consumer.subscribe(&topics.to_vec()).expect("Can't subscribe to specified topics");

    // consumer.start() returns a stream. The stream can be used ot chain together expensive steps,
    // such as complex computations on a thread pool or asynchronous IO.
    println!("try start");
    let mut message_stream = consumer.start();

    while let Some(message) = message_stream.next().await {
        match message {
            Err(e) => println!("Kafka error: {}", e),
            Ok(m) => {
                let payload = m.payload().unwrap();
                let payload: Vec<u8> = payload.iter().cloned().collect();
                match sender.send(payload).await {
                    Ok(()) => {},
                    Err(_) => {
                        // TODO
                    }
                }

                consumer.commit_message(&m, CommitMode::Async).unwrap();
            }
        };
    }
}

pub async fn try_subscribe_topic(broker_addr: &str, group_id: &str, topic: String, sender: Sender<Vec<u8>>, listen_rsp_topics: Arc<Mutex<HashMap<String, String>>>)
{
    {
        let mut map = listen_rsp_topics.lock().await;
        if !map.contains_key(&topic) {
            let broker = String::from(broker_addr);
            let group_id = String::from(group_id);
            map.insert(topic.clone(), topic.clone());
            tokio::spawn(async move {
                let topics = [topic.as_str()];
                consume_and_print(&broker, &group_id, &topics, sender).await;
            });
        }
    }
}

pub async fn produce(brokers: &str, topic_name: &str, data: &Vec<u8>, topic_producer: Arc<Mutex<HashMap<String, FutureProducer>>>) {
    {
        let mut map = topic_producer.lock().await;
        if let Some(producer) = map.get_mut(topic_name) {
            let future = producer
                .send(
                    FutureRecord::to(topic_name)
                        .payload(data)
                        .key(topic_name),
                    0,
                )
                .map(move |status| { });
            future.await;
        } else {
            let producer: FutureProducer = ClientConfig::new()
                .set("bootstrap.servers", brokers)
                .set("message.timeout.ms", "5000")
                .create()
                .expect("Producer creation error");
            let future = producer
                .send(
                    FutureRecord::to(topic_name)
                        .payload(data)
                        .key(topic_name),
                    0,
                )
                .map(move |status| { });
            future.await;
            map.insert(topic_name.to_string(), producer);
        }
    }

}

pub struct TopicMng {
    redis_uri: String,
    redis_client: redis::Client,
    topic_list_key: String,
    room_topic: HashMap<String, String>, // room_id -> topic
    user_room: HashMap<i64, String>,
    user_room_update_time: HashMap<String, i64>, // room_id:user_id -> last update time
}

impl TopicMng {
    pub fn new(redis_uri: String, topic_list_key: String) -> TopicMng {
        return TopicMng {
            redis_uri: redis_uri.clone(),
            redis_client: redis::Client::open(redis_uri.clone()).unwrap(),
            topic_list_key: topic_list_key,
            room_topic: HashMap::new(), // room_id in range(x(i), y(i)) must belong to topic i
            user_room: HashMap::new(),
            user_room_update_time: HashMap::new(),
        };
    }

    pub async fn get_room_topic(&mut self, room_id: &String) -> Option<String> {
        if let Some(v) = self.room_topic.get(room_id) {
            return Some(v.clone());
        }
        let mut conn = self.redis_client.get_async_connection().await.unwrap();
        let key = format!("topic_name:{}:", room_id);
        match conn.get(key).await {
            Ok(v) => {
                match v {
                    redis::Value::Nil => {
                        return None;
                    }
                    _ => {}
                };
                let topic: String = redis::from_redis_value::<String>(&v).unwrap();
                println!("get topic:{} of room_id:{}", room_id, topic);
                self.room_topic.insert(room_id.to_string(), topic.to_string());
                return Some(topic);
            },
            Err(err) => {
                println!("redis get err: {}", err);
                return None;
            }
        }
        return None;
    }

    pub async fn get_free_room_topic(&mut self) -> Option<String> {
        let mut conn = self.redis_client.get_async_connection().await.unwrap();
        match conn.zrange(self.topic_list_key.clone(), 0, 0).await {
            Ok(v) => {
                match v {
                    redis::Value::Nil => {
                        return None;
                    }
                    _ => {}
                };
                let v: Vec<String> = redis::from_redis_value::<Vec<String>>(&v).unwrap();
                return Some(v[0].clone());
            },
            Err(err) => {
                println!("redis get err: {}", err);
                return None;
            }
        }
    }

    pub async fn get_user_room_id(&mut self, user_id: &i64) -> Option<String> {
        if let Some(room_id) = self.user_room.get(user_id) {
            let key = format!("{}:{}", room_id, user_id);
            if let Some(t) = self.user_room_update_time.get(&key) {
                return Some(room_id.clone());
            }
            self.user_room.remove(&user_id);
            self.user_room_update_time.remove(&key);
        }
        let mut conn = self.redis_client.get_async_connection().await.unwrap();
        let key = format!("room_id:{}:", user_id);
        match conn.get(key).await {
            Ok(v) => {
                match v {
                    redis::Value::Nil => {
                        return None;
                    }
                    _ => {}
                };
                let room_id: String = redis::from_redis_value::<String>(&v).unwrap();
                println!("get room_id:{} of user_id:{}", room_id, user_id);
                self.user_room.insert(*user_id, room_id.to_string());
                let key = format!("{}:{}", room_id, user_id);
                self.user_room_update_time.insert(key, 0);
                return Some(room_id);
            },
            Err(err) => {
                println!("redis get err: {}", err);
                return None;
            }
        }
        return None;
    }
}


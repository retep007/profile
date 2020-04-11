use crate::scheduler;
use crate::import::*;
use log::debug;
use tonic::{Request, Response, Status};
use futures::{FutureExt, StreamExt};
use futures::channel::mpsc;

pub use proto::scheduler_server::{Scheduler, SchedulerServer};
use proto::{RegistrationReply, RegistrationRequest};

pub mod proto {
    tonic::include_proto!("scheduler"); // The string specified here must match the proto package name
}
type SchedulerObj = Arc<Mutex<scheduler::Scheduler>>;

pub struct SchedulerService {
    scheduler: SchedulerObj,
}

impl SchedulerService {
    pub fn new(scheduler: SchedulerObj) -> Self {
        Self { scheduler }
    }
}

#[tonic::async_trait]
impl Scheduler for SchedulerService {
    type SubscribeTasksStream = mpsc::Receiver<Result<proto::SubscribeTasksReply,  tonic::Status>>;

    async fn register_server(
        &self,
        request: Request<RegistrationRequest>,
    ) -> Result<Response<RegistrationReply>, Status> {
        let request = request.into_inner();
        debug!("Registering server id: '{}'", request.machine_id);
        self.scheduler.lock().await.insert_server(scheduler::Server::new(request.machine_id, None)).await;

        let reply = proto::RegistrationReply { should_benchmark: true };

        Ok(Response::new(reply))
    }

    async fn submit_benchmark(
        &self,
        request: Request<proto::BenchmarkSubmitRequest>,
    ) -> Result<Response<proto::BenchmarkSubmitReply>, Status> {
        let request = request.into_inner();
        debug!("Received benchmark from server id: '{}'", request.machine_id);
        let profile = request.profile.unwrap();
        let profile = scheduler::ResourceProfile {
            cpu: profile.instructions / profile.cycles,
            disk: profile.vfs_read + profile.vfs_write,
            memory: profile.memory,
            network: profile.tcp_send_bytes + profile.tcp_recv_bytes,
        };

        let server = scheduler::Server::new(request.machine_id, Some(profile));
        debug!("Registering server with profile: '{:?}'", server);
        self.scheduler.lock().await.insert_server(server).await;

        let reply = proto::BenchmarkSubmitReply {};

        Ok(Response::new(reply))
    }

    async fn subscribe_tasks(
        &self,
        request: Request<proto::SubscribeTasksRequest>,
    ) -> Result<Response<Self::SubscribeTasksStream>, tonic::Status> {
        let (sched_tx, sched_rx) = mpsc::channel(10);
        
        let request = request.into_inner();
        self.scheduler.lock().await.subscribe_server(request.machine_id, sched_tx);

        let (tx, rx) = mpsc::channel(10);

        tokio::task::spawn(sched_rx
            .map(|x| {
                let task = Some(x.task.into());
                let state: proto::subscribe_tasks_reply::State = x.state.into();
                let res = proto::SubscribeTasksReply {task, state: state as i32};
                Ok(Ok(res))
            })
            .forward(tx)
            .map(|result| {
                if let Err(e) = result {
                    error!("task send error: {}", e);
                }
            })
        );


        Ok(Response::new(rx))
    }
}


impl From<scheduler::Task> for proto::subscribe_tasks_reply::Task {
    fn from(task: scheduler::Task) -> Self {
        Self {
            image: task.get_image().clone(),
            is_profiled: task.get_request().is_none(),
        }
    }
}

impl From<scheduler::State> for proto::subscribe_tasks_reply::State {
    fn from(state: scheduler::State) -> Self {
        match state {
            scheduler::State::Run => Self::Run,
            scheduler::State::Remove => Self::Remove,
        }
    }
}
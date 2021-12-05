# dcron
**Status**: Design phase

The idea for this project is to create an easy way to create cron-like jobs without having to worry about *where* the script will run. For this to happen a client should be able to talk with any server to schedule a job or retrieve a job result/log. In order to keep the logs accessible by a web-interface, once the job is done 
the log will be uploaded to a object storage service (such as S3, ceph or openstack Swift). The client will also upload the script it wants to run in a object storage service and only give to the server the time it wants it to run (using cron-syntax) and the intepreter the server should use to run it.

The service itself uses a DocumentDB such as https://couchdb.apache.org/ to store its data.

## API

### Communication between client and server
#### API: CREATE JOB
##### Request message
- time: cron job syntax to define the frequency and time of execution
- script type: python or ruby
- script_location: object storage service url/id
##### Response message
- job id

![image](https://user-images.githubusercontent.com/266034/144726107-04c863f3-28c0-402a-8e24-fd6147de3db7.png)

#### API: UPDATE JOB

Exactly like create, but request message has a field for the job id.

#### API: GET JOB EXECUTIONS
##### Request message
- Job id
##### Response Message:
- Array of execution object:
    -  Execution date
    -  Execution result
    -  Logs path

### Communication between leader and workers
The distributed system has an active (leader) node, which is responsible for polling the database every minute. When there is a new job to be executed, it sends to a node (for now based on round-robin). If the worker is busy, it can refuse to execute the job.

There is no need for the worker to communicate back the result of the job*, it just need to update the DocumentDB. The job must have a timeout, in which case, the leader will retry the job again in another node.

* not sure about it, need to think better on edge cases regarding timeouts.

##### API: EXECUTE JOB
Leader node to worker
###### Request message
- job id
- script type
- script location
###### Response message
- Accepted status (declined/accepted)


##### API: SAVE JOB STATUS
Worker to leader node
###### Request message
- Job id
- Log path
- exit status code

## Implementation Details

### Leader election

In order to elect a leader we need to archivie consensus. Paxos is normally my goto tool, but I read about https://raft.github.io/ and I want to try it out.

The leader is only need to coordinate which machine is going to run which job. The machines can talk directly to the documentDB to save result of jobs and to insert new jobs.

The Execution document at the Database is append-only, meaning that if due to timeout two machines execute the same job, both executions will be kept at the database.

### Web Interface

The web interface will have very few features, it can see the jobs, their executions, and logs. It won't be possible for now to change any data about the job itself. The idea is that most of the configuration should be done always using the client. In the future the idea is to have a way to template the jobs, similar to Terraform.


### Language

I'm still not sure if I will go with Go or Rust. I plan to use gRPC for the communication between computers, and it seems like there is not great support for gRPC in the Rust community (may be wrong). Go would be fine for the service, given that there's no heavy CPU bound operation. Running the scripts itself should be a task for the OS.


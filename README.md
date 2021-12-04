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
##### API: EXECUTE JOB
Leader node to worker
###### Request message
- job id
- script type
- script location
###### Response message
- Accepted status (declined/accepted)

![image](https://user-images.githubusercontent.com/266034/144726349-b2335169-e460-4044-bbcb-83cab267bc2f.png)

##### API: SAVE JOB STATUS
Worker to leader node
###### Request message
- Job id
- Log path
- exit status code

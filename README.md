# Kafka-to-HTTP

## An open source alternative to the confluent http sink connector

This alternative operates differently than the connector in that's deployed outside of your kafka cluster, and doesn't require you to install the connector on all of your brokers. This is advantageous if you're using a managed kafka service and don't want to have to, well, manage your managed service.


## Why use an http-proxy from kafka?
Eventually if your monolith gets too big you will inevitably either try to start decomposing it in to smaller services, or just start creating new services outside of the monolith, and have inter-service communication over http when you need it. Then, what will inevitably happen is that one of those services will go down, which means anything along that path of requests is actually down too, and sure enough your whole app is down. The next logical step is to put a queue inbetween the two critial services that are communicating with eachother and there will be a few "problems"/hurdles to get over:
1. Lots of the popular queuing solutions today are proprietary and only exist in some cloud provider (AWS SQS, Google PubSub), and maybe you want to do your best to avoid vendor lock-in
2. You built your company/product up until this point on what you're good at, building webservers. But a queue-consumer is a much different resource pattern with tons of caveats, do you have this experience already at the company?
3. You already have an http webserver built to accept this request, why change it to a queue consumer?

The aim of this project is to help make decoupling your services with kafka as easy as possible, with a one click deployment of kafka-to-http with a configuration file.

## How it works
This services acts as an http-proxy by sitting "infront" of a kafka topic, subscribing to all of the partitions, consuming messages as they come, and making an http request to configured endpoint(s).

## Configuration
- Exactly once delivery, your message will be read from kafka and delievered to an http endpoint once and only once if the request is successful. Any subsequent message deliveries are a result of a retry policy
- Dead Letter Queue (DLQ) to be able to inspect failed jobs (we're no longer just dropping messages like we used to, yay!)
- Endpoint(s) as configuration, and we can have many! Have an event that multiple services need to react to? No problem. Adding more endpoints will affect throughput since it now takes _n_ times the latency of your endpoints to consume a message.
- Concurrency, control the rate that you're proxying events, no more services going down because of an intermitent spike in traffic
- Retryability, define all the retry characteristics you'd expect, retryable status codes, max attempts, timeouts etc.

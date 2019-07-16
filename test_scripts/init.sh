#! /bin/bash
rabbitmq-server &disown
amqp-declare-queue -u amqp://rustmqtt:rustmqtt@localhost:5672 -q rust_gateway
mosquitto&disown
 
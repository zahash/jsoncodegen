#!/bin/bash
set -e

mvn clean package
mvn dependency:copy-dependencies -DoutputDirectory=target/lib -DincludeScope=runtime
java -jar ./target/jsoncodegen-1.0.jar

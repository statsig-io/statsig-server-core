FROM gradle:8-jdk11 AS builder
RUN apt-get update && apt-get install -y jq
WORKDIR /app
COPY build.gradle ./
RUN gradle dependencies --no-daemon
COPY . .

# This next line prevent caching of the lines below it
ADD "https://www.random.org/cgi-bin/randbyte?nbytes=10&format=h" skipcache

RUN ./update_java_core.sh
RUN gradle shadowJar --no-daemon
RUN cat build/versions.properties
RUN  gradle dependencies --configuration compileClasspath

FROM eclipse-temurin:11
WORKDIR /app
COPY --from=builder /app/build/libs/*.jar app.jar
COPY --from=builder /app/build/versions.properties /app/build/versions.properties
ENV JAVA_OPTS="-XX:+UseG1GC -Xms512m -Xmx2g"
CMD ["java", "-jar", "app.jar"]

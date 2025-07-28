FROM gradle:8-jdk11 AS builder
WORKDIR /app
COPY build.gradle ./
RUN gradle dependencies --no-daemon
COPY . .
RUN gradle shadowJar --no-daemon
RUN cat build/versions.properties

FROM eclipse-temurin:11
RUN java --version
WORKDIR /app
COPY --from=builder /app/build/libs/*.jar app.jar
COPY --from=builder /app/build/versions.properties /app/build/versions.properties
ENV JAVA_OPTS="-XX:+UseG1GC -Xms512m -Xmx2g"
CMD ["java", "-jar", "app.jar"]

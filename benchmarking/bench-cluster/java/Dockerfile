FROM gradle:8-jdk11 AS builder
ARG BUILD_FOR_CI
WORKDIR /app
COPY build.gradle ./
RUN gradle -PBUILD_FOR_CI="${BUILD_FOR_CI}" dependencies --no-daemon
COPY . .
RUN gradle -PBUILD_FOR_CI="${BUILD_FOR_CI}" shadowJar --no-daemon

FROM eclipse-temurin:11
RUN java --version
WORKDIR /app
COPY --from=builder /app/build/libs/*.jar app.jar
COPY --from=builder /app/build/versions.properties /app/build/versions.properties
ENV JAVA_OPTS="-XX:+UseG1GC -Xms512m -Xmx2g"

# Run the application
CMD ["java", "-jar", "app.jar"]

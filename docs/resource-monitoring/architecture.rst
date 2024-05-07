Resource Monitoring Architecture
================================

This architecture document contains info on the backend and frontend architectures.

Backend
-------

We deploy the resource monitoring stack using docker-compose.
This stack contains 1) Grafana 2) Prometheus 3) Nginx.
Note: the stack has been set to auto restart through docker-compose.

Nginx is used as a reverse proxy to group the grafana endpoints and the testbed server dashboard dashboard.
This is because we embed the grafana graphs as iframes inside the testbed dashboard.
Since the localhost URLs are using different ports, we run into CORS problems.
So to avoid changing the browser settings to reduce security, we put everything under a reverse proxy.

The testbed server provides endpoints for prometheus to scrape metrics, these are grouped under host and guests.
In these endpoints, the testbed server (master) will collect metrics on the active testbed hosts, then the guests in all active deployments.
Every testbed, both master and client will have endpoints that provide the metrics for itself and the guests it is running.
The master will work out where everything is, based on the state.json for each active deployment and call the respective testbed host.
Prometheus will scrape the master testbed host's metrics endpoint periodically (5s) to collect the metrics for every eligible host/guest.
Note: the metric scraping endpoint is only enabled on the testbed server when in master mode.

To get metrics such as CPU time, we need to sample the cpu time twice.
Given we have a limit of 5s between prometheus scraping metrics, we are just sampling between 0.5s on each endpoint request.
This does mean the metrics can be a little inaccurate.
While this can be tuned, we need to consider the time it takes for the master testbed host to poll every testbed for each guest.
Therefore the tuning must consider how this scales as we add more testbed hosts and more guests to the testbed cluster.


Libvirt
~~~~~~~

A connection to the libvirt daemon is made to request metric data.

Docker
~~~~~~

We look directly at the filesystem under `/sys/fs/cgroup/system.slice/docker-<container id>.scope/` for live metrics.
Docker offers a `stats` endpoint, but this uses a 1s sample rate.
Rather than connecting to the unix socket over and over for this, we have opted to directly inspect the files.
This means we are at least consistent in the sampling rate of all guest types.

Android
~~~~~~~

N/A


Frontend
--------

The testbed server resource monitoring dashboard endpoint takes in a named deployment.
It will look at the state.json and get the names of the testbed hosts and guests that make up the deployment, then render an html page requesting graphs from grafane for each host/guest.
These graphs are embedded in an iframe

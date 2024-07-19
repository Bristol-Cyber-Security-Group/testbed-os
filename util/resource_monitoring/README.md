# Testbed Resource Monitoring

The database backend for the testbed resource monitoring is a Prometheus DB running via docker compose.
This just makes it easy to control the state of the server, when the testbed is restarted and if the user does not want/need resource monitoring.

Prometheus works in a pull based system, where prometheus will do the periodic scraping on the specified endpoints to get data.
This means we offer an endpoint on the testbed server that will respond with the data for prometheus to store.

These services will just continuously run in the background and will restart with the main.

## Security 

The grafana admin credentials can be found in the `docker-compose.yaml` file.

The dashboards are given anonymous access, so you can access them without logging in.

## File Mounts

### Prometheus 

The prometheus data is stored in a docker volume `prometheus_data`.

The prometheus configuration file `prometheus.yaml` is mounted to the expected directory `/etc/prometheus/prometheus.yml`

### Grafana

We mount the grafana configuration to point it to the Prometheus DB, and also the testbed's pre-made dashboards.


## Time Series and Labels

We have a series of time series, where each time series is labelled for each guest or host.
Hosts are just labelled with their name, and guests are labelled with their name and the project they belong to.

## Dashboards

The guest dashboard is given a variable `proj_n` which is the project/deployment name.
This means the dashboards are parameterised to get just the specific guests for that project/deployment.

For example:

`http://localhost:3000/d/c74125df-061c-43fd-9eae-3ae6c8d0d37e/my-dashboard?orgId=1&var-proj_n=signal`

This will get the guest resource data for the `signal` project.
Note the variable is prepended with `var-` which is required by grafana.


Host dashboard: `http://localhost:3000/d/a501b9c3-2632-4862-b91c-00d9575a6ba3/testbed-host-resources?orgId=1`

Guest dashboard (with the signal project): `http://localhost:3000/d/c74125df-061c-43fd-9eae-3ae6c8d0d37e/my-dashboard?orgId=1&var-proj_n=signal`

## Reverse Proxy

Since we are only using this on localhost, we need to avoid browser cross-origin resource sharing issues so we use a reverse proxy.
This proxy will serve the testbed servers dashboard and grafana under one URL.

The testbed server's dashboard is on `/api/metrics/dashboard` but on the reverse proxy it is just under `/resource-monitoring`.

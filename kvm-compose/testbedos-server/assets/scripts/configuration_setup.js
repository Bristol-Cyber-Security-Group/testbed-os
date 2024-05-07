$(document).ready(function() {

    fill_current_host_json_values();
    fill_current_qemu_conf_values();
    fill_current_resource_monitoring_state();

    $('#setupConfigHostJsonSubmit').click(function () {
        let alert = $('#setupHostJsonAlertPlaceholder');
        alert.empty();

        // get the current json
        let json_request = new XMLHttpRequest();
        // TODO - this should be async, need to introduce awaits
        json_request.open("GET", server_url + '/api/config/host?pretty=true', false);
        json_request.send(null);
        let json = JSON.parse(json_request.responseText);

        let new_user = $('#setupConfigUser').val();
        let new_main_interface = $('#setupConfigMainInterface').val();

        // do some validation
        if (new_user !== "") {
            json["user"] = new_user;
        } else {
            alert.append(add_alert("danger", "New user can't be empty"));
            return;
        }
        if (new_main_interface !== "") {
            json["main_interface"] = new_main_interface;
        } else {
            alert.append(add_alert("danger", "New main interface can't be empty"));
            return;
        }

        // post the new json file
        $.ajax({
            url: server_url + '/api/config/host',
            type: 'POST',
            data: JSON.stringify(json),
            contentType: 'application/json',
            success: function (json) {
                // update current
                alert.append(add_alert("success", "Update successful, see updated current values."));
                fill_current_host_json_values();
            },
            error: function (error) {
                console.log(error);
                alert.append(add_alert("danger", "There was a problem updating, see console or raise an issue."));
            }
        });
    });

    $('#setupConfigQemuConfSubmit').click(function () {
        let alert = $('#setupHostQemuConfPlaceholder');
        alert.empty();
    });

});

function fill_current_host_json_values() {
    $.ajax({
        url: server_url + '/api/config/host?pretty=true',
        type: 'GET',
        success: function (json) {
            $('#setupConfigUserCurrent').attr("value", json["user"]);
            $('#setupConfigMainInterfaceCurrent').attr("value", json["main_interface"]);
        },
        error: function (error) {

        }
    });
}

function fill_current_qemu_conf_values() {

    let alert = $('#setupHostQemuConfPlaceholder');
    alert.empty();

    $.ajax({
        url: server_url + '/api/config/usergroupqemu',
        type: 'GET',
        success: function (json) {
            let json_parsed = JSON.parse(json)
            $('#setupConfigQemuUserCurrent').attr("value", json_parsed["user"]);
            $('#setupConfigQemuGroupCurrent').attr("value", json_parsed["group"]);
        },
        error: function (error) {
            alert.append(add_alert("danger", "There was a problem getting qemu conf values."));
        }
    });
}

function fill_current_resource_monitoring_state() {
    let alert = $('#setupResourceMonitoringAlertPlaceholder');
    alert.empty();

    $.ajax({
        url: server_url + '/api/metrics/state',
        type: 'GET',
        success: function (json) {
            let grafana_badge = $('#setupConfigGrafanaBadge');
            let prometheus_badge = $('#setupConfigPrometheusBadge');
            let nginx_badge = $('#setupConfigNginxBadge');

            grafana_badge.removeClass();
            prometheus_badge.removeClass();
            nginx_badge.removeClass();

            let parsed_json = JSON.parse(json);
            if (parsed_json["grafana"] === true) {
                grafana_badge.addClass("badge");
                grafana_badge.addClass("text-bg-success");
                grafana_badge.html("Grafana Running");
            } else {
                grafana_badge.addClass("badge");
                grafana_badge.addClass("text-bg-danger");
                grafana_badge.html("Grafana Off");
            }

            if (parsed_json["prometheus"] === true) {
                prometheus_badge.addClass("badge");
                prometheus_badge.addClass("text-bg-success");
                prometheus_badge.html("Prometheus Running");
            } else {
                prometheus_badge.addClass("badge");
                prometheus_badge.addClass("text-bg-danger");
                prometheus_badge.html("Prometheus Off");
            }

            if (parsed_json["nginx"] === true) {
                nginx_badge.addClass("badge");
                nginx_badge.addClass("text-bg-success");
                nginx_badge.html("Nginx Running");
            } else {
                nginx_badge.addClass("badge");
                nginx_badge.addClass("text-bg-danger");
                nginx_badge.html("Nginx Off");
            }
        },
        error: function (error) {
            console.log(error);
            alert.append(add_alert("danger", "There was a problem getting resource monitoring state."));
        }
    });
}

function add_alert(alert_type, alert_text) {
    let icon = "";

    if (alert_type === "success") {
        icon = "    <svg width=\"24\" height=\"24\" fill=\"currentColor\" class=\"bi bi-check-circle-fill flex-shrink-0 me-2\" viewBox=\"0 0 16 16\" role=\"img\" aria-label=\"Warning:\"><use xlink:href=\"#check-circle-fill\"/></svg>\n\n";
    }
    if (alert_type === "danger") {
        icon = "    <svg width=\"24\" height=\"24\" fill=\"currentColor\" class=\"bi bi-exclamation-triangle-fill flex-shrink-0 me-2\" viewBox=\"0 0 16 16\" role=\"img\" aria-label=\"Warning:\"><use xlink:href=\"#exclamation-triangle-fill\"/></svg>\n\n";
    }

    return "<br>" +
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"d-none\">\n" +
        "  <symbol id=\"check-circle-fill\" viewBox=\"0 0 16 16\">\n" +
        "    <path d=\"M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zm-3.97-3.03a.75.75 0 0 0-1.08.022L7.477 9.417 5.384 7.323a.75.75 0 0 0-1.06 1.06L6.97 11.03a.75.75 0 0 0 1.079-.02l3.992-4.99a.75.75 0 0 0-.01-1.05z\"/>\n" +
        "  </symbol>\n" +
        "  <symbol id=\"info-fill\" viewBox=\"0 0 16 16\">\n" +
        "    <path d=\"M8 16A8 8 0 1 0 8 0a8 8 0 0 0 0 16zm.93-9.412-1 4.705c-.07.34.029.533.304.533.194 0 .487-.07.686-.246l-.088.416c-.287.346-.92.598-1.465.598-.703 0-1.002-.422-.808-1.319l.738-3.468c.064-.293.006-.399-.287-.47l-.451-.081.082-.381 2.29-.287zM8 5.5a1 1 0 1 1 0-2 1 1 0 0 1 0 2z\"/>\n" +
        "  </symbol>\n" +
        "  <symbol id=\"exclamation-triangle-fill\" viewBox=\"0 0 16 16\">\n" +
        "    <path d=\"M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566zM8 5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 5.995A.905.905 0 0 1 8 5zm.002 6a1 1 0 1 1 0 2 1 1 0 0 1 0-2z\"/>\n" +
        "  </symbol>\n" +
        "</svg>\n" +
        `<div class=\"alert alert-${alert_type} alert-dismissible fade show\" role=\"alert\">\n` +
        icon +
        `    ${alert_text}\n` +
        "    <button type=\"button\" class=\"btn-close\" data-bs-dismiss=\"alert\" aria-label=\"Close\"></button>\n" +
        "</div>";
}

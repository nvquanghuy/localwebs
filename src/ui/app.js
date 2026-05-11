let autoRefreshInterval;

async function fetchServices() {
    try {
        const response = await fetch('/api/services');
        const services = await response.json();
        displayServices(services);
    } catch (error) {
        console.error('Failed to fetch services:', error);
        showError();
    }
}

function displayServices(services) {
    const grid = document.getElementById('services-grid');
    const loading = document.getElementById('loading');
    const emptyState = document.getElementById('empty-state');

    loading.style.display = 'none';

    if (services.length === 0) {
        grid.innerHTML = '';
        emptyState.style.display = 'block';
        return;
    }

    emptyState.style.display = 'none';
    grid.innerHTML = services.map(service => createServiceCard(service)).join('');

    // Add click listeners to cards
    document.querySelectorAll('.service-card').forEach((card, index) => {
        card.addEventListener('click', () => {
            // Use the same hostname/IP that user accessed the portal with
            const serviceUrl = buildServiceUrl(services[index].port);
            window.open(serviceUrl, '_blank');
        });
    });
}

function buildServiceUrl(port) {
    // Use the current hostname instead of hardcoded 127.0.0.1
    const hostname = window.location.hostname;
    return `http://${hostname}:${port}`;
}

function createServiceCard(service) {
    const statusClass = service.is_healthy ? 'status-healthy' : 'status-unhealthy';
    const responseTime = service.response_time_ms
        ? `${service.response_time_ms}ms`
        : 'N/A';

    const processInfo = service.process_name
        ? `<div class="info-row">
               <span class="info-label">Process:</span>
               <span>${service.process_name}${service.pid ? ` (${service.pid})` : ''}</span>
           </div>`
        : '';

    const title = service.title
        ? `<div class="info-row">
               <span class="info-label">Title:</span>
               <span>${escapeHtml(service.title)}</span>
           </div>`
        : '';

    const description = service.description
        ? `<div class="info-row">
               <span class="info-label">Description:</span>
               <span>${escapeHtml(service.description)}</span>
           </div>`
        : '';

    return `
        <div class="service-card">
            <div class="service-header">
                <div class="service-name">${escapeHtml(service.name)}</div>
                <div class="status-indicator ${statusClass}"></div>
            </div>
            <div class="service-port">:${service.port}</div>
            <div class="service-info">
                ${description}
                ${processInfo}
                ${title}
                <div class="info-row">
                    <span class="info-label">Response:</span>
                    <span>${responseTime}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">Source:</span>
                    <span class="source-badge">${service.source}</span>
                </div>
            </div>
        </div>
    `;
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function showError() {
    const loading = document.getElementById('loading');
    loading.innerHTML = '<p style="color: white;">Failed to load services. Please try again.</p>';
}

function startAutoRefresh() {
    autoRefreshInterval = setInterval(fetchServices, 10000);
}

function stopAutoRefresh() {
    if (autoRefreshInterval) {
        clearInterval(autoRefreshInterval);
    }
}

document.getElementById('refresh-btn').addEventListener('click', async () => {
    document.getElementById('loading').style.display = 'block';
    document.getElementById('services-grid').innerHTML = '';
    await fetchServices();
});

// Initial load
fetchServices();

// Start auto-refresh
startAutoRefresh();

// Stop auto-refresh when page is hidden
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        stopAutoRefresh();
    } else {
        startAutoRefresh();
    }
});

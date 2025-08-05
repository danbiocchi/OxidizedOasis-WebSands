-- Clean up all test databases
DO $$
DECLARE
    db_name TEXT;
BEGIN
    -- Terminate all connections to test databases
    FOR db_name IN 
        SELECT datname FROM pg_database 
        WHERE datname LIKE 'test_oxidizedoasis_db_%'
    LOOP
        EXECUTE format('SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = %L AND pid <> pg_backend_pid()', db_name);
    END LOOP;
    
    -- Wait briefly for connections to close
    PERFORM pg_sleep(1);
    
    -- Drop all test databases
    FOR db_name IN 
        SELECT datname FROM pg_database 
        WHERE datname LIKE 'test_oxidizedoasis_db_%'
    LOOP
        EXECUTE format('DROP DATABASE IF EXISTS %I', db_name);
        RAISE NOTICE 'Dropped database: %', db_name;
    END LOOP;
END $$;